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

/// **Глиф** — квадрат галки и круг радиокнопки.
///
/// ⚠ **Здесь лежит то, что у трёх тел ВПРАВДУ одинаково, и не лежит то, что различается.**
/// Правило «три тела» решения 107.2: сливать только одну таблицу. Таблица ролей цвета
/// (`theme::glyph_color_roles`), размер клетки, радиус скругления и вынос точки у окна настроек
/// (`draw_glyph_element`), мастера (`draw_wizard_glyph`) и переключателя «От автора»
/// (`draw_switch`) — общие с самого начала. Различаются:
///
/// * **земля.** Настройки спрашивают, лежит ли элемент на панели, и грунтуют панельной кистью;
///   мастер грунтует фоном окна всегда.
/// * **подпись.** Настройки берут её у контрола и рисуют `DrawTextW` с отступом
///   `GLYPH_TEXT_INSET_DLU` **от левого края элемента**; мастер получает её параметром и рисует
///   `theme::paint_label` (островки RTL, задача Т-30-4) с отступом **от правого края клетки**.
/// * **рамка фокуса.** У настроек она есть, у мастера её нет.
/// * **перо круга.** Настройки при `frame == None` рисуют эллипс БЕЗ пера, мастер — пером цвета
///   заливки. Это расходится на пиксель, и слить их — значит подвинуть пиксель в окне, которое
///   этап обязан оставить неизменным (решение 107.2).
///
/// Поэтому сюда переехала **клетка**: единственная арифметика, тождественная во всех трёх.
/// Остальное названо выше и оставлено — это ответ на вопрос «что не слилось и чем различалось».
pub mod glyph {
    use super::RECT;
    use crate::settings::GLYPH_SIZE;
    use crate::theme::scaled;

    /// Квадрат (или круг) элемента: у левого края, по центру по вертикали, [`GLYPH_SIZE`]
    /// пикселей макета в стороне через масштаб — `$bs = 17` генератора и его же `$by = $py +
    /// [int](($ph - $bs)/2)` для центровки (задача T-11-16).
    ///
    /// Перенесено из `settings::draw_glyph_element` и `letters::draw_wizard_glyph` задачей
    /// Т-45-2: тела считали это **одинаково**, слово в слово, и теперь считают одним телом.
    pub fn cell(rect: RECT, dpi: i32) -> RECT {
        let side = scaled(GLYPH_SIZE, dpi);
        let top = rect.top + (rect.bottom - rect.top - side) / 2;

        RECT {
            left: rect.left,
            top,
            right: rect.left + side,
            bottom: top + side,
        }
    }

    /// **Род элемента, отвечающего на нажатие** — решение 109.1, задача Т-46-2.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Kind {
        /// Галка: нажатие переключает её саму на обратное.
        Check,
        /// Радиокнопка: нажатие выбирает её из ряда, и ряд перерисовывается целиком.
        Radio,
        /// Карточка и кнопка: нажатие делает то, что за элементом написано, — и только оно.
        Button,
    }

    /// **Что окно обязано сделать по одному уведомлению `WM_COMMAND`** — решение 109.1.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Answer {
        /// Переключить состояние на обратное — ответ [`Kind::Check`].
        Toggle,
        /// Выбрать этот элемент в ряду — ответ [`Kind::Radio`].
        Choose,
        /// Сделать то, что за элементом написано, — ответ [`Kind::Button`].
        Act,
        /// Ничего: это уведомление состояния не меняет.
        Nothing,
    }

    /// **Одно правило нажатия на все окна программы** — решение 109.1, задача Т-46-2.
    ///
    /// Тело перенесено из `settings::restore_self_switching` (задача T-11-5b, FR-92а), где
    /// оно работало с 2026-08; поведение окна настроек не меняется ни на пиксель, а мастер
    /// «Написать автору» и «От автора» встают на него.
    ///
    /// # ⛔ Почему фильтр вообще нужен: замер Т-46-1
    ///
    /// Все глифы этой программы — `BS_OWNERDRAW | BS_NOTIFY`, и `BS_NOTIFY` не украшение: без
    /// него `BN_SETFOCUS` не приходит вовсе, а он нужен ходу по ряду радио стрелками. Цена —
    /// **на один настоящий щелчок мышью приходит очередь из трёх уведомлений**, и замер её
    /// напечатал (`scratchpad-Э46\красное-галки-e45.log`):
    ///
    /// ```text
    /// щелчок по галке 1338, фокус был на 1337:
    ///     1337/BN_KILLFOCUS, 1338/BN_SETFOCUS, 1338/BN_CLICKED
    /// повторный щелчок по 1338, фокус уже на ней:
    ///     1338/BN_CLICKED
    /// ```
    ///
    /// Окно, которое отвечает на **каждое** уведомление, переключает соседку (её
    /// `BN_KILLFOCUS`) и переключает свою **дважды** — то есть не переключает вовсе. Ровно это
    /// увидел пользователь: «то не ставятся с первого раза, то не снимаются с первого раза».
    /// Замер на 44 щелчках: мастер — 36 несработавших и 35 сдвинутых соседок, окно настроек с
    /// этим фильтром — **0 и 0** при той же очереди уведомлений.
    ///
    /// # Почему у радио есть вторая дорога
    ///
    /// Радиокнопка отвечает и на приход фокуса, **который вызвала клавиша-стрелка**: так ходят
    /// по `WS_GROUP`-ряду, и так вёл себя автоматический тип, которому эта отрисовка пришла на
    /// смену. [`arrow_is_down`] синхронизирован с очередью, поэтому ответ — состояние на миг
    /// того нажатия, которое сейчас разбирается.
    pub fn answer(kind: Kind, notification: u16) -> Answer {
        use windows::Win32::UI::WindowsAndMessaging::{BN_CLICKED, BN_DBLCLK, BN_SETFOCUS};

        let notification = u32::from(notification);
        let clicked = notification == BN_CLICKED || notification == BN_DBLCLK;

        match kind {
            Kind::Check if clicked => Answer::Toggle,
            Kind::Radio if clicked || (notification == BN_SETFOCUS && arrow_is_down()) => {
                Answer::Choose
            }
            Kind::Button if clicked => Answer::Act,
            _ => Answer::Nothing,
        }
    }

    /// Нажата ли сейчас одна из четырёх стрелок — тех клавиш, которыми менеджер диалога ходит
    /// по ряду `WS_GROUP`.
    ///
    /// Перенесено из `settings::arrow_key_is_down` задачей Т-46-2, тело не менялось.
    /// `key_is_down` синхронизирован с очередью — нужное состояние есть состояние на миг того
    /// нажатия, которое сейчас разбирается.
    fn arrow_is_down() -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{VK_DOWN, VK_LEFT, VK_RIGHT, VK_UP};

        [VK_LEFT, VK_UP, VK_RIGHT, VK_DOWN]
            .into_iter()
            .any(crate::settings::key_is_down)
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
/// настроек и `letters::field_box_air` мастера. Т-45-2 сложил их в одно тело с параметром
/// `OddPixel`, а Т-46-5 убрал и параметр: стандарт один — коробка равна заказанной высоте
/// (решение 109.6).
pub mod field {
    use super::RECT;

    /// Воздух над контролом и под ним, чтобы коробка вокруг него встала `box_height` высотой.
    ///
    /// # ⭐ Один стандарт без вариантов — решение 109.6, задача Т-46-5
    ///
    /// **Коробка равна заказанной высоте, нечётный остаток — вниз.** Слово пользователя
    /// (2026-09-05, дословно): «нужен общий вариант, мы же приводим все к одному стандарту, а
    /// не подстраиваемся под мастера».
    ///
    /// До Э46 здесь стоял параметр `OddPixel` с двумя значениями: мастер с задачи Т-33а-2 клал
    /// лишний пиксель **вниз** и получал ровно заказанные 12 единиц диалога, а окно настроек с
    /// задачи T-12-3 делило остаток поровну и лишний пиксель **теряло** — замер Т-46-1 на этой
    /// машине: **25 px вместо 26** (`scratchpad-Э46\красное-коробки-e45.log`). Параметра больше
    /// нет, и окно настроек получило 26: это единственная намеренная перемена его пикселей за
    /// этап, и она названа решением.
    ///
    /// `None` — `MapDialogRect` отказал (NFR-13): тогда рамка стоит на одной толщине, ровно как
    /// стояла у каждого поля до задачи T-12-3. То же и когда контрол уже выше коробки.
    ///
    /// Перенесено из `theme::field_frame_air` и `letters::field_box_air` задачей Т-45-2; два
    /// тела сведены в одно задачей Т-46-5.
    pub fn air(box_height: Option<i32>, control_height: i32, border: i32) -> (i32, i32) {
        let Some(box_height) = box_height else {
            return (border, border);
        };

        let extra = box_height - control_height;

        if extra < border * 2 {
            return (border, border);
        }

        // Делится пополам, а остаток — вниз: сумма равна `extra` в точности, поэтому коробка
        // выходит ровно той высоты, которую назвал заказ.
        let above = extra / 2;
        (above, extra - above)
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
        invalidate(hwnd, control, true);
    }

    /// То же, но **без стирания фона перед рисованием** — задача Т-45-3, находка прибором.
    ///
    /// ⛔ **Стирание и есть тот кадр, в котором элемента нет.** Owner-draw кнопка на
    /// `WM_ERASEBKGND` спрашивает у родителя кисть (`WM_CTLCOLORBTN`), а родитель отвечает
    /// кистью **фона окна**: карточка мастера на один кадр заливается фоном окна и только потом
    /// приходит `WM_DRAWITEM`, который рисует её целиком. Пойман снимком на живом продукте:
    /// на худшем кадре первой карточки в окне НЕТ ВОВСЕ, расхождение 27 700 пикселей.
    ///
    /// Элементу, который рисует **весь свой прямоугольник** сам — а таковы и карточки, и глифы,
    /// и подписи этой программы, — стирание не нужно ни для чего. `bErase = FALSE` убирает
    /// промежуточный кадр, не меняя ни одного устоявшегося пикселя.
    ///
    /// ⚠ Дверь отдельная, а не флаг у общей: окно настроек ходит прежней дорогой и остаётся при
    /// своём поведении до пикселя и до кадра (решение 107.2).
    pub fn control_no_erase(hwnd: HWND, control: i32) {
        invalidate(hwnd, control, false);
    }

    fn invalidate(hwnd: HWND, control: i32, erase: bool) {
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
        let _ = unsafe { InvalidateRect(Some(window), None, erase) };
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
