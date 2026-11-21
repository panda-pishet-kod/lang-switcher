//! **Один слой элементов на все окна программы** — решение 107.1, задача Т-45-2.
//!
//! Слово пользователя, которым этот модуль заведён (2026-09-04, дословно): «У нас уже есть
//! проверенные решения, которые работают в основном окне, почему здесь индивидуальное решение,
//! а не подстановка уже готовых механизмов — я думаю, это неправильно. Механизм работы для всех
//! окон должен быть однотипным и просто распределяться там, где нужно, а тот визуал-функционал,
//! которого где-то нам не хватает по каким-то причинам, нужно просто добавить к общему
//! механизму — это решит многие проблемы и централизованно упростит работу».
//!
//! # Что здесь живёт
//!
//! Глиф (галка и радиокнопка), комбобокс (подкласс, высоты, тело строки), кнопка, рамка поля,
//! подпись и точечная перерисовка — **одним телом на элемент**. Всё, что рисуется в окнах
//! `settings`, «О программе», «От автора» и в мастере FR-104, рисуется отсюда.
//!
//! # Три границы, и они не размыты
//!
//! * **`theme` — краски и примитивы.** Палитры, кисти, [`crate::theme::PaintBuffer`],
//!   `paint_rounded`, `paint_ellipse`, `paint_label`, таблицы ролей цветов. У `theme` нет
//!   `HWND` и нет представления о том, что такое «контрол».
//! * **`widgets` — элементы.** Здесь есть `HWND`, есть идентификатор контрола и есть знание о
//!   том, как элемент устроен: где у галки квадрат, какой высоты строка списка, где стоит
//!   рамка поля. Красок здесь нет — они приходят разрешёнными.
//! * **Окна — раскладка, хранилище и ворота.** Где элемент стоит, какое у него состояние и
//!   **чей это идентификатор** (SEC-05), решает окно. Слой рисует ровно то, что окно назвало
//!   своим, и **состояние приходит параметром**: `widgets` не знает ни про `DialogState`, ни
//!   про черновик мастера, ни про `AuthorView`.
//!
//! Чего слою не хватает — добавляется **в слой**, а не рядом с ним.
//!
//! # ⛔ FR-72
//!
//! Голого `SendMessageW` в этом модуле нет и быть не может: сторож
//! `tests\guard.rs::send_message_is_used_nowhere_and_the_timeout_is_the_fifty_of_fr72`
//! запрещает его по всей `src\`. Посылки идут через [`crate::settings::send_to`], у которого
//! есть верхняя граница ожидания.

use windows::Win32::Foundation::HWND;

/// **Клавиши полей ввода, одни на все окна программы** — задачи T-118-2 и T-118-6.
pub mod keys {
    use super::HWND;
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::MSG;

    /// How long a send to the field waits — a window of this very thread, so the call is direct and
    /// the number is only the bound FR-72 asks of every send.
    const FIELD_TIMEOUT_MS: u32 = 50;

    /// **Ctrl+A в поле ввода окна программы выделяет весь текст** — задача T-118-2 (мастер
    /// «Написать автору», Q-208-23 п. 3, обращение SKHCQ3SA) и задача T-118-6 (поле «Вручную» окна
    /// настроек, слово владельца 208.80 «Сейчас, в 1.6.1»).
    ///
    /// ⛔ **Поле `EDIT` этой программы Ctrl+A не знает вовсе — ни однострочное, ни многострочное.**
    /// Программа не берёт `comctl32` шестой версии (в манифесте его нет), и поле — классическое
    /// `EDIT` из `user32`. Замер на стенде с 1.6.0: Ctrl+A не выделил ничего ни в одном из семи полей
    /// мастера и в «Вручную» окна настроек (`reports\ITOG-E118.md`). Менеджер диалога этой клавиши
    /// тоже не знает, поэтому её берёт это тело **до** `IsDialogMessageW`.
    ///
    /// # Одно тело, две дороги
    ///
    /// Его зовёт `letters::filter_message` — фильтр окон писем в цикле сообщений потока и в
    /// перехватчике модального цикла `letters::ModalLoopFilter`. Окно настроек модальное, и его
    /// сообщения идут только через перехватчик: так тело достаётся и ему, без копии (до T-118-6 оно
    /// жило в `letters` и брало только поля окон писем).
    ///
    /// # Что берётся
    ///
    /// Только `WM_KEYDOWN` клавиши A при зажатом Ctrl — и **без Alt и без Shift**: Ctrl+Alt — это
    /// AltGr, на польской раскладке AltGr+A печатает «ą», и такой набор должен дойти до поля как
    /// есть. Только окно **этого потока** — окна программы, а не чужое окно, чей `HWND` пришёл в
    /// сообщении. И только **поле ввода** — то, что само говорит, что понимает `EM_SETSEL`:
    /// `DLGC_HASSETSEL` в ответе на `WM_GETDLGCODE`, тем же вопросом поле узнаёт менеджер диалога.
    /// Список раскладок, список исключений и комбо этого ответа не дают и живут как прежде; поле
    /// горячей клавиши отвечает `DLGC_STATIC` вне захвата и `DLGC_WANTALLKEYS` в захвате — Ctrl+A там
    /// остаётся захвату. Имя класса не читается: `class_name` — звено цепочки захвата, которую задача
    /// T-100-2 увела в `guard`. Нажатие съедается целиком: `WM_CHAR` с кодом 1, который из него
    /// сделал бы `TranslateMessage`, полю не нужен.
    ///
    /// `EM_SETSEL(0, −1)` уходит полю этого же потока через `SendMessageTimeoutW` — FR-72 п. 17
    /// запрещает голый `SendMessage` во всём `src\`.
    ///
    /// Public for the windows of `letters` and `settings`, and for `tests\settings.rs`.
    pub fn select_all_on_ctrl_a(message: &MSG) -> bool {
        use windows::Win32::System::Threading::GetCurrentThreadId;
        use windows::Win32::UI::Controls::EM_SETSEL;
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            GetKeyState, VIRTUAL_KEY, VK_A, VK_CONTROL, VK_MENU, VK_SHIFT,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            DLGC_HASSETSEL, GetWindowThreadProcessId, SMTO_ABORTIFHUNG, SendMessageTimeoutW,
            WM_GETDLGCODE, WM_KEYDOWN,
        };

        if message.message != WM_KEYDOWN || message.wParam.0 != usize::from(VK_A.0) {
            return false;
        }

        // SAFETY: `GetKeyState` takes a virtual-key code and reads the key state of this thread's
        // queue as of the message being handled; no memory of ours is touched.
        let held = |key: VIRTUAL_KEY| unsafe { GetKeyState(i32::from(key.0)) < 0 };

        if !held(VK_CONTROL) || held(VK_MENU) || held(VK_SHIFT) {
            return false;
        }

        let field: HWND = message.hwnd;

        // SAFETY: a handle out of the message, passed by value; the call answers the thread that
        // made the window, or zero for a handle that names no window — which is not this thread.
        let thread = unsafe { GetWindowThreadProcessId(field, None) };

        // SAFETY: takes nothing and answers the identifier of the calling thread.
        if field.is_invalid() || thread != unsafe { GetCurrentThreadId() } {
            return false;
        }

        let mut code = 0_usize;

        // NFR-13: a refused send is «not a field» — the key then goes on to the dialog manager, as
        // it always did.
        //
        // SAFETY: `field` is a window of this thread; the message carries no pointer, and `code`
        // is a live local the call writes the answer into.
        let answered = unsafe {
            SendMessageTimeoutW(
                field,
                WM_GETDLGCODE,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                FIELD_TIMEOUT_MS,
                Some(&mut code),
            )
        }
        .0 != 0;

        if !answered || code & DLGC_HASSETSEL as usize == 0 {
            return false;
        }

        // NFR-13: examined in words and dropped — the field is a window of this thread, the send is
        // a direct call of its procedure, and a refusal leaves the selection as it was, which is all
        // the key ever did before T-118-2.
        //
        // SAFETY: `field` is a field of this thread; the message carries two plain numbers.
        let _ = unsafe {
            SendMessageTimeoutW(
                field,
                EM_SETSEL,
                WPARAM(0),
                LPARAM(-1),
                SMTO_ABORTIFHUNG,
                FIELD_TIMEOUT_MS,
                None,
            )
        };

        true
    }
}
