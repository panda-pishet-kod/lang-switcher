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

use windows::Win32::Foundation::{HWND, RECT};
use windows::core::Error as WinError;

/// Единицы диалога окна в пикселях — `MapDialogRect`, документированный переводчик.
///
/// Перенесено из `settings.rs` задачей Т-45-2, тело не менялось. Место ему здесь: единица
/// диалога — это мера ЭЛЕМЕНТОВ окна, и на ней стоит вся геометрия слоя. Окно настроек зовёт
/// её по имени (`use crate::widgets::dialog_units;`), поэтому все его вызовы читаются как
/// читались.
///
/// `None` при отказе (NFR-13: разобрано; каждый вызывающий тогда остаётся при своём умолчании).
pub fn dialog_units(hwnd: HWND, horizontal: i32, vertical: i32) -> Option<(i32, i32)> {
    use windows::Win32::UI::WindowsAndMessaging::MapDialogRect;

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: horizontal,
        bottom: vertical,
    };

    // SAFETY: `hwnd` is the live dialog and `rect` is a live local the call rewrites in
    // place; nothing else is written.
    unsafe { MapDialogRect(hwnd, &mut rect) }.ok()?;

    Some((rect.right, rect.bottom))
}

/// **Комбобокс** — высоты и подкласс, одни на все окна (решение 107.3).
///
/// ⛔ **Ради чего это здесь.** У `CBS_OWNERDRAWFIXED` одно `WM_MEASUREITEM` задаёт высоту и
/// закрытой части, и КАЖДОЙ строки выпадающего списка, поэтому их разводят двумя разными
/// числами: строку списка ставит ответ на сообщение, закрытую часть — `CB_SETITEMHEIGHT(−1)`
/// после создания окна. Мастер FR-104 взял у окна настроек подкласс, а высоты — нет: он
/// отвечал `body_height + 8` без масштаба DPI и не звал `CB_SETITEMHEIGHT` вовсе. Замер на
/// живом продукте 0.44.0: строка списка мастера **16 px** против **26 px** у всех четырёх комбо
/// окна настроек. Это и есть «нестандартный для моей программы размер» из слов пользователя.
///
/// Поэтому [`attach`] ставит закрытую высоту **сам**: забыть её теперь нельзя.
pub mod combo {
    use super::{HWND, dialog_units};
    use crate::settings::{DIALOG_FONT_HEIGHT_DLU, send_to};
    use crate::theme::{dc_dpi, scaled};
    use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
    use windows::Win32::UI::WindowsAndMessaging::{CB_ERR, CB_SETITEMHEIGHT};

    /// Воздух над строкой выпадающего списка и под ней, в пикселях макета — задача T-11-15.
    ///
    /// Перенесено из `settings::COMBO_LIST_ITEM_EXTRA` задачей Т-45-2, значение не менялось.
    pub const LIST_ITEM_EXTRA: i32 = 12;

    /// То же для ЗАКРЫТОЙ части — задача T-12-2, FR-92а.
    ///
    /// Через [`scaled`] двойка возвращается единицей при 96 DPI, и закрытая часть встаёт
    /// 15 + 1 + 6 = 22 px против 22,5 макета. Отдельная константа, а не меньшая
    /// [`LIST_ITEM_EXTRA`], именно потому, что одно `WM_MEASUREITEM` красит обе высоты сразу:
    /// уменьшить ответ на сообщение значило бы ужать и строки списка.
    ///
    /// Перенесено из `settings::COMBO_CLOSED_ITEM_EXTRA` задачей Т-45-2, значение не менялось.
    pub const CLOSED_ITEM_EXTRA: i32 = 2;

    /// `wParam` сообщений `CB_SETITEMHEIGHT` и `CB_GETITEMHEIGHT`, называющий **поле выбора**
    /// (закрытую часть) вместо строки списка: документированная −1.
    const SELECTION_FIELD: usize = usize::MAX;

    /// Одна строка комбобокса: высота шрифта диалога плюс `air` пикселей макета через масштаб.
    ///
    /// Высота шрифта берётся документированным `MapDialogRect`: восемь вертикальных единиц
    /// диалога — это по определению одна высота шрифта, поэтому прямоугольник в
    /// `DIALOG_FONT_HEIGHT_DLU` единиц отображается ровно в неё; ни одного дескриптора шрифта
    /// не передаётся и ни одного сообщения не посылается.
    ///
    /// `None` при отказе `MapDialogRect` (NFR-13: разобрано; оба вызывающих тогда оставляют
    /// контролу ту высоту, что у него была).
    ///
    /// Перенесено из `settings::combo_row_height` задачей Т-45-2, тело не менялось.
    pub fn row_height(hwnd: HWND, air: i32) -> Option<i32> {
        let (_, font_height) = dialog_units(hwnd, 0, DIALOG_FONT_HEIGHT_DLU)?;

        // SAFETY: `hwnd` is the dialog — a live window either being built (`WM_MEASUREITEM`) or
        // filled in (`WM_INITDIALOG`); the call answers its DC or an invalid handle, and the DC
        // is released below on both paths.
        let dc = unsafe { GetDC(Some(hwnd)) };

        // NFR-13: examined — see the doc comment above.
        let extra = scaled(air, dc_dpi(dc));

        if !dc.is_invalid() {
            // SAFETY: releases exactly the DC taken above, once.
            unsafe { ReleaseDC(Some(hwnd), dc) };
        }

        Some(font_height + extra)
    }

    /// Приводит ЗАКРЫТУЮ часть одного комбобокса к 12 единицам диалога макета — FR-92, FR-92а,
    /// задача T-12-2.
    ///
    /// Отвечает, взял ли контрол высоту. `CB_ERR` — отказ: комбобокс остаётся при прежней,
    /// более высокой закрытой части, что есть видимая деградация, а не молчаливая (NFR-13);
    /// словарь `diag` для такого отказа строки не имеет (reviews\T-11-1.md).
    ///
    /// Перенесено из `settings::set_combo_closed_height` задачей Т-45-2: тело то же, только
    /// цикл по четырём комбо окна настроек остался у окна — слой отвечает за ОДИН элемент.
    pub fn set_closed_height(hwnd: HWND, control: i32) -> bool {
        let Some(height) = row_height(hwnd, CLOSED_ITEM_EXTRA) else {
            return false;
        };

        let answer = send_to(
            hwnd,
            control,
            CB_SETITEMHEIGHT,
            SELECTION_FIELD,
            isize::try_from(height).unwrap_or(0),
        );

        answer != CB_ERR as isize
    }

    /// Ставит общий подкласс на комбобокс окна и **тут же задаёт ему закрытую высоту**.
    ///
    /// ⭐ Две вещи в одном вызове — не удобство, а лечение: мастер взял подкласс и забыл высоту,
    /// и забыть её теперь негде. Всякое окно, которому нужен комбобокс программы, зовёт это.
    ///
    /// ⚠ Сама процедура подкласса (`combo_box_proc`) пока живёт в `settings.rs` вместе с
    /// рисованием закрытой части, которое читает состояние того окна; сюда переехал **адрес**
    /// механизма, а не его тело. Перевозить тело — отдельная работа, и она названа в отчёте
    /// (правило «три тела»).
    pub fn attach(hwnd: HWND, control: i32) -> bool {
        crate::settings::subclass_foreign_combo(hwnd, control);
        set_closed_height(hwnd, control)
    }

    /// Снимает подкласс — дальняя половина пары [`attach`].
    pub fn detach(hwnd: HWND, control: i32) {
        crate::settings::unsubclass_foreign_combo(hwnd, control);
    }
}

/// **Коробка вокруг однострочного поля** — одна арифметика на все окна (решение 107.4).
///
/// Рамку поля рисует **фон окна**, а не подкласс контрола, и это общий механизм с окном настроек
/// (задача T-12-3): однострочный `EDIT` кладёт текст по верху своей клиентской области, и
/// подвинуть его оттуда нечем — `EM_SETRECT` документирован для многострочных, `EM_SETMARGINS`
/// двигает бока. Поэтому контролу дают **одну кегельную строку**, а коробку в 12 (окно настроек)
/// или 14 (мастер) единиц диалога рисуют вокруг него.
///
/// Сюда сведены две арифметики, которые до Т-45-2 жили порознь: `theme::field_frame_air` окна
/// настроек и `letters::field_box_air` мастера. Тела не менялись — они сложены в одно с
/// **явным** параметром [`OddPixel`], потому что различались ровно им.
pub mod field {
    use super::RECT;

    /// Куда девается лишний пиксель, когда остатка на воздух не хватает на две равные половины.
    ///
    /// ⛔ **Два значения тут не от лени, а потому что окна вправду ведут себя по-разному, и
    /// этап Э45 не вправе это менять.** Мастер после задачи Т-33а-2 кладёт лишний пиксель
    /// **вниз** — тогда коробка выходит ровно той высоты, которую назвал макет. Окно настроек
    /// с задачи T-12-3 делит остаток поровну и лишний пиксель **теряет**: при базовой единице
    /// этой машины его коробка выходит 25 px вместо 26. Это записано и в его тесте словами
    /// «centring may lose the odd pixel of an odd remainder and nothing more».
    ///
    /// Свести их в одно значение — значит подвинуть пиксель в окне, которое этап обязан
    /// оставить неизменным до пикселя (решение 107.2). Различие оставлено, названо и вынесено
    /// пользователю вопросом (`reports\ИТОГ-Э45.md`, раздел «Я»).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum OddPixel {
        /// Лишний пиксель уходит **вниз** — мастер, задача Т-33а-2.
        Below,
        /// Лишний пиксель **теряется** — окно настроек, задача T-12-3.
        Dropped,
    }

    /// Воздух над контролом и под ним, чтобы коробка вокруг него встала `box_height` высотой.
    ///
    /// `None` — `MapDialogRect` отказал (NFR-13): тогда рамка стоит на одной толщине, ровно как
    /// стояла у каждого поля до задачи T-12-3. То же и когда контрол уже выше коробки.
    ///
    /// Перенесено из `theme::field_frame_air` и `letters::field_box_air` задачей Т-45-2; обе
    /// арифметики сохранены в точности, различие названо параметром.
    pub fn air(
        box_height: Option<i32>,
        control_height: i32,
        border: i32,
        odd: OddPixel,
    ) -> (i32, i32) {
        let Some(box_height) = box_height else {
            return (border, border);
        };

        let extra = box_height - control_height;

        if extra < border * 2 {
            return (border, border);
        }

        match odd {
            OddPixel::Below => {
                let above = extra / 2;
                (above, extra - above)
            }
            OddPixel::Dropped => {
                let half = extra / 2;
                (half, half)
            }
        }
    }

    /// Прямоугольник коробки вокруг контрола: воздух сверху и снизу, толщина по бокам.
    ///
    /// ⛔ **Место под кромку обязана оставить РАСКЛАДКА, а не эта функция.** Коробка выходит за
    /// левый край контрола на `thickness`, и если сосед слева стоит вплотную — он эту кромку
    /// затрёт: подпись «Программа:» в мастере грунтует свой прямоугольник целиком и стирала
    /// левую кромку рамки (замер Т-45-1: 4 пикселя кромки из 19). Решение 107.4: раскладка
    /// резервирует `thickness` плюс зазор.
    pub fn frame(control: RECT, air: (i32, i32), thickness: i32) -> RECT {
        RECT {
            left: control.left - thickness,
            top: control.top - air.0,
            right: control.right + thickness,
            bottom: control.bottom + air.1,
        }
    }
}

/// **Точечная перерисовка** — то, чем окно настроек обходится там, где мастер стирал окно
/// целиком (решение 107.5).
///
/// Перенесено из `settings.rs` задачей Т-45-2, тела не менялись.
pub mod repaint {
    use super::{HWND, WinError};
    use windows::Win32::Graphics::Gdi::{
        InvalidateRect, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RedrawWindow,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetDlgItem;

    /// Repaints one control now — task T-11-5b: writing check state changes no pixels of an
    /// owner-drawn button, so `set_check` asks for the repaint the moment it writes the
    /// store (task T-11-5b-2).
    ///
    /// Перенесено из `settings::repaint_control` задачей Т-45-2, тело не менялось.
    pub fn control(hwnd: HWND, control: i32) {
        // SAFETY: `hwnd` is the live dialog and `control` names a control of its template;
        // the crate turns a missing control into an error, which is the `Ok` guard below.
        let Ok(window) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
            crate::app::report_non_critical("GetDlgItem", &WinError::from_thread());
            return;
        };

        // NFR-13: examined in words and deliberately dropped — the call refuses only for a
        // window that is not alive, this one was found the line above, and the journal has no
        // row for GDI refusals (reviews\T-11-1.md).
        //
        // SAFETY: `window` is the live control just found; a null rectangle means its whole
        // client area, and the call keeps no pointer.
        let _ = unsafe { InvalidateRect(Some(window), None, true) };
    }

    /// Repaints every control of one contiguous identifier range — the radio ranges of task
    /// T-11-5b, the same runs `check_radio` walks in the store.
    ///
    /// Перенесено из `settings::repaint_control_range` задачей Т-45-2, тело не менялось.
    pub fn range(hwnd: HWND, first: i32, last: i32) {
        for id in first..=last {
            control(hwnd, id);
        }
    }

    /// Repaints a window of this program and every child in it, background included.
    ///
    /// Two occasions, one body. **A palette change** — `refresh_palette` for the settings dialog
    /// and `refresh_about_palette` for the about window (task T-13-17) — and, since task Т-23-5,
    /// **arming or ending a capture**: the frame of the hotkey field is drawn by the window's own
    /// background and changes colour with the capture, and that background is a cached picture
    /// which has to be painted again for the change to be seen.
    ///
    /// ⚠ **Это дорогая дорога, и она остаётся дорогой смены палитры и смены шага** — не дорогой
    /// щелчка по элементу. Щелчок перерисовывает затронутое (решение 107.5): именно
    /// стирание окна целиком на каждый щелчок по карточке и было тем морганием, ради которого
    /// заведён этот модуль.
    ///
    /// Перенесено из `settings::repaint_whole_window` задачей Т-45-2, тело не менялось.
    pub fn whole(hwnd: HWND) {
        // NFR-13: both answers are examined in words and deliberately dropped. Either call
        // refuses only for a window that is not alive, and `hwnd` is the dialog whose
        // procedure is running; there is nothing to do about a refused invalidation beyond the
        // next `WM_PAINT` arriving anyway, and the journal has no row for it
        // (reviews\T-11-1.md).
        //
        // SAFETY: `hwnd` is the live dialog; a null rectangle means the whole client area, and
        // neither call keeps a pointer.
        let _ = unsafe { InvalidateRect(Some(hwnd), None, true) };

        // `RDW_ALLCHILDREN` is the half `InvalidateRect` does not reach: the controls repaint
        // too, so the person who pressed «Применить» sees one whole dialog in the new palette,
        // not a new background behind stale controls.
        //
        // SAFETY: as above.
        let _ = unsafe {
            RedrawWindow(
                Some(hwnd),
                None,
                None,
                RDW_ERASE | RDW_INVALIDATE | RDW_ALLCHILDREN,
            )
        };
    }
}
