# Changelog

Every published version of Lang Switcher, newest first. The full text of each release — the installer's size and
SHA-256, what was measured and what was not — is on the [Releases](https://github.com/panda-pishet-kod/lang-switcher/releases)
page. Dates are the days the releases were published (UTC).

## 1.6.1 — 2026-11-16

*The settings window no longer blinks, «Apply» is grey without changes, and Ctrl+A and Tab work in the wizard.*

- The settings window no longer blinks when you press «Set» next to the hotkey or «Cancel» to stop: only the frame of the
  key field and the button are redrawn, not the whole window. The same when a key is accepted, when the capture is cancelled
  by Esc or a click elsewhere, and when «Apply» is pressed in the middle of a capture — 1.6.0 then left the button saying
  «Cancel».
- «Apply» in the settings window is grey while the settings on the screen are the ones applied: the window opens with it
  grey, any change lights it, putting the value back greys it again, and after the press the focus moves to «OK».
- In the fields of the «Write to the author…» wizard Ctrl+A selects the whole text, in every field.
- Tab and Shift+Tab move between the fields and buttons of the wizard also when it was opened from the settings window. In
  1.6.0 they did not move there, and in the multi-line fields Tab typed a tab character.
- What did not change: the default hotkey, the look of the windows, the interface languages, the settings and their file,
  where to write to the author.

## 1.6.0 — 2026-11-15

*The layout switches in the wizard on a button too, and the knock sounds only for a real refusal.*

- The program's own «Write to the author…» wizard: the layout switches when the focus is on a button too (after «Next», for
  example). 1.5.1 switched it only in the text box.
- When a busy window takes the change of layout a little late, the refusal sound no longer plays. On a press with no word the
  program watches for up to half a second whether the layout changed and answers by what happened: changed — the double click,
  not changed — the dull knock.
- In a program on the exclusion list the hotkey stays silent even when it is pressed the moment that program comes to the front.
- A window that came to the front without the program being told is checked at the first key of a word — for a password field
  and for the exclusion list.
- The installer started from a PowerShell 7 terminal installs the full version. In that case 1.5.1 silently installed the base one,
  which does not work in the windows of programs run as administrator.
- The name of the program and the download button open the pages of the site in Swedish and Slovak.
- The request for the news file names the version of the program in its header — `LangSwitcher/1.6.0`, and the first read after an
  installation adds `?first=1` to the address once; the request carries nothing about the computer, as before.
- What did not change: the default hotkey, the look of the windows, the interface languages, the settings and their file, where to
  write to the author.

## 1.5.1 — 2026-11-13

*The layout switches in the Windows Run dialog too.*

- The request to switch the layout now goes to the box with the keyboard focus, not to the frame of the window. The frame of
  the Windows Run dialog (`Win`+`R`) did not pass that request on: 1.5.0 corrected the word but left the layout as it was, and a
  press with no word did not change the layout either — on Windows 10 and Windows 11 alike. Now the layout switches there too.
- The program's own «Write to the author…» wizard: in its text box the word is corrected and the layout switches — users of
  1.4.4 wrote about this window.
- What did not change: everything else — the default hotkey, the look of the windows, the interface languages, the settings and
  their file, where to write to the author.

## 1.5.0 — 2026-11-12

*The hotkey switches the layout, password fields in browsers and WinForms, site pages in nineteen languages.*

- With nothing typed and nothing selected, the hotkey now switches the keyboard layout to the next one of the program's cycle: you
  can pick the language ahead with `Pause` instead of `Alt+Shift`. Press again to go on, or back with two layouts. A new, sixth line
  of «How to use» in the About window says so. Such a press used to change nothing.
- In a password field the hotkey only switches the layout and never converts what was typed. In a program on the exclusion list it
  still stays silent and does nothing.
- In 1.4.4 and earlier the password field was not recognised in some programs and sometimes in browsers: never in WinForms programs,
  and in Edge, Chrome and Firefox not in a field reached by `Tab`. What was typed there was kept in the buffer like in an ordinary
  field, and the hotkey could convert it. These fields are recognised now: the program listens to the focus events of UI Automation.
  What is typed into a password field does not enter the buffer, and the press is not slower.
- A program on the exclusion list started again while Lang Switcher runs was sometimes not recognised: Windows brought its window
  forward late, and a word in it was converted. The program now checks the window in front again and recognises the exclusion.
- The name of the program and the «Open the download page» button open the pages of the site in nineteen interface languages. New:
  Hebrew, Greek, Azerbaijani, Bulgarian, Serbian (Cyrillic and Latin), German, French, Turkish, Portuguese, Dutch, Polish, Czech,
  Italian, Hungarian. Armenian and Uzbek open the Russian pages.
- The examples of a wrong-layout word are corrected for Bulgarian (`zdrawej` → «здравей») and Dutch (`wqterdqg` → «zaterdag», the
  Belgian layout).
- The installer speaks Greek. The wording of the Hebrew, Greek and Azerbaijani translations, the Brazilian forms of Portuguese, the
  consistency of Czech and the button named in the Turkish setup wizard are corrected.
- What did not change: where to write to the author, the default hotkey, the look of the windows apart from the sixth line, the set
  of interface languages, the settings and their file.

## 1.4.4 — 2026-11-01

*The bare grave key as the language switcher, site pages in four languages.*

- If the language in Windows is switched by the bare grave key, as is common in Thailand, the program now treats the key as a switcher.
  It used to count as a typed character and spoiled the text around it: `hello กดส` came out as `hello_กดส`, the rollback by a second
  press was gone, and a letter was replaced by the grave instead. Now the word after the key converts correctly, the text around it stays
  intact and the rollback works. The same for `Shift`+grave: with this assignment Windows switches on it too.
- To find this out the program reads three values of the keyboard settings of Windows (`HKCU\Keyboard Layout\Toggle`). It reads nothing
  else in that key and writes nothing there. If the key is not assigned, it stays an ordinary character, as before.
- With the interface in Ukrainian, Arabic, Persian or Thai, the name of the program and the «Open the download page» button now open
  the pages of the site in that language. Ukrainian used to land on the Russian page, the other three on the English one.
- The site has no «What's new» page in these languages yet. A click on the update notification goes to the English page and does
  not disturb the language chosen on the site.
- Arabic: the numeral in the line under the news is corrected («ثلاثة أخبار»).
- Unchanged: where to write to the author, the addresses of the pages in the other languages, the hotkey, the look of the windows, the
  set of interface languages.

## 1.4.3

Not released. Its one change — the pages of the site in Ukrainian, Arabic and Persian — came out in 1.4.4.

## 1.4.2 — 2026-10-31

*Notepad, quotation marks of the language and the layout pair.*

- Notepad of Windows 11: a typed word is now selected by its position in the text and not by `Shift+Left` presses. Notepad passes the
  Tamil conjunct `க்ஷ` with one caret stop or with three, and no count is right for both: the replacement either ate the space before the
  word or left a piece in front of it (`X க்Ta`). The field is sent two positions only; the program does not read its text.
- Notepad of Windows 11, the `backspace` replacement method (configuration file only): the replacement now goes by the selection,
  like the default method. After `Backspace` Notepad lost the first character of the insertion, repeated its last character, and took a
  Tamil consonant with its pulli off sometimes with one press, sometimes with two.
- The program's windows now quote the way the language does: English, Turkish and Portuguese “…”, German and Czech „…“, Polish
  „…”. The same marks in the line of the installer about the «No» button in German, Czech, Polish, Turkish and Portuguese. The Ukrainian
  strings got their calques and mixed words fixed («папка журналу», «програма встановлення») and the apostrophe.
- The emergency chord `Ctrl+Alt+Shift+F12` no longer reaches the window under it: Notepad used to replace the last character with its
  code (`X ghbdtn` became `X ghbdt6E`). It still works with the program hung.
- Configuration file: `pair_source` and `pair_target` accept the identifier of a variant of a language (`0x00020422`). Such a
  value used to find no layout in silence, and the first two layouts of the system worked in its place. If the variant is not
  in the system, nothing is replaced and the journal gets the line `pair variant layout missing`.
- The layout pair is looked for among the first 64 layouts of the system, not 8: the settings window used to show the ninth as
  valid while the hotkey did nothing. The «Several layouts» mode still walks at most eight.
- Unchanged: the hotkey, the look of the windows, the set of interface languages, the default replacement method.

## 1.4.1 — 2026-10-30

*More exact with special layouts and in Notepad.*

- A letter the target layout builds from two dead keys in a row is now built the way Windows builds it: in French BÉPO `´`,
  `´`, `o` is `ő`, not `´´o`.
- Canadian Multilingual Standard: a word typed with the right `Ctrl` (the extra level of that layout) converts into the same
  layout of another language of Canada, with the character of the same level. A layout without such a level — every other
  layout of Windows — still does not take the word, and the program sounds a refusal.
- Notepad with the Tamil 99 layout: the replacement no longer lands after the word with nothing erased.
- Notepad, the `backspace` replacement method (configuration file only): a Tamil consonant with its pulli, typed by keys, no
  longer leads to erasing the character before the word. Notepad takes such a pair off sometimes with one press and sometimes
  with two, so a consonant may stay in front of the replacement — visible and easy to remove.
- Unchanged: the hotkey, the look of the windows, the interface languages, the configuration file.

## 1.4.0 — 2026-10-29

*The interface in 22 more languages, 66 in all.*

- The interface in 22 more languages: Vietnamese, Dutch, Romanian, Hungarian, Swedish, Albanian, Danish, Finnish, Croatian,
  Slovak, Norwegian (Bokmål and Nynorsk), Lithuanian, Slovenian, Bosnian, Latvian, Estonian, Welsh, Maltese, Icelandic,
  Luxembourgish and Māori — 66 in all.
- On its first start the program speaks the language of Windows; a Norwegian Windows in Bokmål or Nynorsk gets its own
  written norm, Bosnian comes in the Latin script.
- The windows in Dutch, Luxembourgish, Welsh and Māori may open one font step smaller on small screens, so that the whole
  window fits; the windows of the earlier languages look as before.
- In Hindi and Urdu the lines of the "State" panel about refusals now say plainly that they are about choosing a layout.
- The installer speaks the 66 languages too; the Hungarian installer puts the names of its buttons in Hungarian quotes.
- What did not change: the hotkey, the look of the windows in the earlier languages, the author's signature. The settings
  file gets a new schema mark (10): if you go back to 1.3.0, it reads the file and leaves it as it is.

## 1.3.0 — 2026-10-29

*The interface in 30 more languages, 44 in all.*

- The interface in 30 more languages: Hindi, Bengali, Urdu, Punjabi, Marathi, Telugu, Tamil, Persian, Gujarati, Thai,
  Kannada, Odia, Malayalam, Uzbek (Latin and Cyrillic), Nepali, Assamese, Azerbaijani, Khmer, Kazakh, Serbian (Cyrillic and
  Latin), Uyghur, Bulgarian, Armenian, Lao, Belarusian, Tatar, Georgian and Macedonian — 44 in all. Urdu, Persian and Uyghur
  read right to left, as Hebrew and Arabic do.
- On its first start the program speaks the language of Windows; Serbian and Uzbek are chosen by the script of Windows.
- The "Interface language" list of the settings: the language of Windows, English, the languages of your keyboard layouts and
  Russian on top, the rest under a line, by their own names.
- The windows of languages with tall scripts (Tamil, Malayalam and eight more) may open one font step smaller on small
  screens, so that the whole window fits; the windows of the 14 earlier languages look as before.
- The welcome letter shows an example in every language — a word typed in the English layout and what it becomes.
- The installer speaks the 44 languages too.
- In Arabic and Hebrew the "Acting cycle" / "Acting pair" line of the settings reads in the right order whatever the names of the layouts (in 1.2.0 it ran backwards with names in Arabic or Hebrew); an Arabic list takes the Arabic comma «،».
- The note about the wizard in the "From the author" window is accurate now: the wizard sends the message itself when you
  press "Send".
- What did not change: the hotkey, the look of the windows in the earlier languages, the author's signature. The settings
  file gets a new schema mark (9): if you go back to 1.2.0, it reads the file and leaves it as it is.

## 1.2.0 — 2026-10-29

*Write to the author with one button, letter statistics of six more languages.*

- Write to the author right from the program. The last step of the "Write to the author" wizard now has a **Send** button:
  the message goes to the author's site — no email app or browser needed, only when you press it yourself, and exactly the
  text you saw. Under the text there is an optional field for how to reach you. If the site does not answer, the wizard
  stays open: **Retry**, the same text by email to `lang.switcher@panda-pishet-kod.dev`, or **Save to the log folder**.
- Letter statistics of six more languages: Arabic, Persian, Indonesian, Hindi, Tamil and Sinhala. Selected text of a
  layout with an ambiguous ligature key, typed not just now, is read by the statistics of the language it is converted to.
  The source is the Leipzig Corpora Collection (Universität Leipzig, CC BY; the Sinhala corpus is drawn from Wikipedia,
  CC BY-SA); only tables of letter frequencies are in the program, not the texts.
- Four rare cases of special layouts. A selected `ґ` of the Ukrainian (Enhanced) layout is converted correctly. The right
  `Alt` with a dead key of another layout (French BÉPO) is a letter, not a command, and the word is no longer cut short.
  The Tamil conjunct `க்ஷ` typed with one key of the Tamil 99 layout is now erased whole in Notepad. A word typed with the
  level of the right `Ctrl` of the Canadian Multilingual Standard layout is not garbled: the program refuses with a sound.
- The description of the program on GitHub is in English now, and its count of languages is exact: 102 languages — 107 combinations of a language and a script (96, plus 11 smaller scripts).
- What did not change: the hotkey, the look of the windows and the settings of the program (except the last step of the
  wizard), the settings file (schema 8), the author's signature.

## 1.1.0 — 2026-10-27

*107 languages listed, letter statistics of French, Spanish and Russian.*

- The list of languages. The description of the program now names every language it works with, as measured on the
  layouts of Windows 11: 107 — 96 living languages and 11 small scripts, from Russian and English to Thai, Hebrew and
  Cherokee. The program itself limits no language except Chinese, Japanese and Korean (layouts with a candidate window).
- Letter statistics of French, Spanish and Russian. Selected text of a layout with an ambiguous ligature key (Arabic: `لا`
  is `B`, or `G` and `H`), typed not just now, is now read by the statistics of the language it is converted to — not only
  English: French `bien` typed in the Arabic layout comes back as `bien`, not `ghien`; Russian `пир` as `пир`, not `ппрр`.
- The `backspace` method in Notepad of Windows 11. If this method is chosen in the settings (the default is `auto`),
  Notepad now and then turned the insertion into its last letter repeated (`uuuuuu` instead of `l;ylfu`). Now the program
  erases the word, waits 50 ms and only then types the replacement.
- The "From the author…" button of the About window speaks the language of the interface — it used to be in Russian in
  every language.
- What did not change: the hotkey, the look of the windows and the settings of the program, the settings file (schema 8),
  the author's signature.

## 1.0.0 — 2026-10-27

*Thai, Hindi, Vietnamese, Polish, French and Arabic without losses.*

- Scripts with marks above and below the letter. In Thai, Hindi, Vietnamese and similar scripts the program used to erase
  too much: a Thai word was replaced together with the space and the character before it. Now the replacement takes
  exactly the word — as many letters as Windows sees in it (Thai `สวัสดี` is 4 letters, Hindi `नमस्ते` is 3).
- The right `Alt` in the English layout. Polish `mąka` typed in the English layout with the keys of Polish (Programmers),
  Kyrgyz `сөз` and other letters on the right `Alt` are no longer lost: such a key press is a letter, not a command. The
  left `Alt`, and the right `Alt` with keys that give no letter, are still commands.
- Dead keys. `être` typed in the English layout with the keys of French (`[etre`) becomes `être`, and `ê` goes back to
  `[e` — the way Windows itself would type them. The same for Spanish `canción` and Portuguese `não`.
- Arabic and other layouts with a ligature key. Selected `لاث` (`be` typed in the Arabic layout) now becomes `be`, not
  `ghe`: text you have just typed is converted by the keys you really pressed, pasted text by letter statistics of English.
  The key presses are kept for this only for such layouts, only in memory, for no longer than a minute and until you move
  to another field; for Russian and English nothing changes — nothing is kept.
- An update now checks that the new version really works: the program's window has to appear and stay. Otherwise the
  previous version is put back.
- What did not change: the hotkey, the look of the windows and the settings of the program, the settings file (schema 8),
  the author's signature.

## 0.104.0 — 2026-10-27

*The program updates itself and starts on a clean Windows.*

- The program updates itself. Once a day, Lang Switcher reads a short news file from the author's site and sends nothing
  about your computer. When a new version comes out, the program downloads it from the same site on its own, checks the
  author's signature and installs it while the computer is idle. After an update, a short notification appears by the
  tray icon; click it to open the "What's new" page.
- How it works. The downloading and installing are done not by the program itself but by a separate file without a window,
  `LangSwitcherUpdate.exe` — the "Lang Switcher Update" scheduled task, once a day and when you sign in to Windows, and only
  while the computer is idle. While it installs, the program closes for a short while and then starts again. The installer comes only from the author's site; its size and SHA-256 must match the ones
  the author signed, and its version must be newer than the installed one. If the new version does not start, the previous
  one is put back.
- The program starts on a Windows without the Visual C++ package. Versions 0.90.0 to 0.103.0 needed `VCRUNTIME140.dll`
  and did not start at all on a clean Windows ("System Error"); everything the program needs is now built into it.
- The news file is read once a day instead of once every 15 days.
- The "Update" letter and the "Version … is available" menu item remain as a fallback: they appear only if a new version
  has not installed itself within three days.
- What did not change: the hotkey, the conversion, the look of the windows and the settings of the program, the settings
  file (schema 8), the author's signature.

## 0.103.0 — 2026-10-19

*The wizard of an appeal leads where you can write.*

- The "Write to the author…" wizard leads where you can write. A report of a problem — it holds the program's journal and
  details of your computer — it offers to send to the author by email: the address is on the screen, with the buttons
  "Copy the text" and "Copy the address"; the program sends nothing and opens no mail program. A suggestion carries only
  your text and the program's version, and its button copies the text and opens a place to discuss it — with the Russian
  or Ukrainian interface, the Lang Switcher topic in the author's Telegram group; with the other twelve languages, the
  program's GitHub Issues. The wizard used to open the author's channel, where a reader has nowhere to write.
- The "Thank you" letter (a month after installing) asks you to say how you like the program in the same place — GitHub
  Issues or the group — and its button "Open GitHub Issues" leads there. "Open the channel" in "What's new" and in the
  "From the author" window still opens the channel: the news is there.
- The pause between the events of a replacement (`inter_event_delay_ms` — in the settings file only, 0 by default) is now
  at most 250 ms instead of 1000: since version 10 1709 Windows gives a keyboard hook no more than a second, and a pause
  of a second was the whole of it. A value above 250 in the file acts as 250; the file is not rewritten. If you never
  touched this setting, nothing changed for you.
- In the "From the author" window in English and Hebrew, the line about the settings file now says the feed can be turned
  off there, not that it is off.
- The "Open the download page" button and a click on the program's name open the site's page in the language of the program:
  the Russian page with the Russian or Ukrainian interface, the English one with the others. They used to open the Russian
  page whatever the language, and the site moved it to the language of the browser or to the one chosen with its button.
- What did not change: the hotkey, the conversion, the look of the windows and the settings of the program, the settings
  file (schema 8), the author's signature, the installer.

## 0.102.0 — 2026-10-16

*The example in the welcome letter speaks the reader's language.*

- The "Hello" letter, which the program shows at its first start, shows its example in the language of the interface: a
  word of your language typed with the English layout on, and how it becomes the word you meant once the hotkey is
  pressed. In German — `Gr[-e` → "Grüße", in French — `sqlut` → "salut", in Ukrainian — `ghbdsn` → "привіт"; in Hebrew
  and Arabic the Latin letters typed turn into the whole word. In English and Polish the example goes the other way:
  "руддщ" → "hello"; in Russian it is what it was, `ghbdtn` → "привет". The caption under the example names the same pair.
- The English strings of the program use American spelling: "recognized", "grayed", "behavior".
- What did not change: the hotkey, the conversion, the windows and settings of the program, the settings file (schema 8),
  the author's signature, the installer.

## 0.101.0 — 2026-10-15

*The name without "for Windows", and uninstalling no longer leaves the log behind.*

- The program is called simply "Lang Switcher", without "for Windows": that is now the name of the setup wizard, the Start
  menu folder, the entry in Apps and the description of the signature the UAC prompt shows. The program and its windows
  were called so before as well.
- The certificate the installer makes on your computer is now called "Lang Switcher (local)". An update removes the old
  one ("Lang Switcher for Windows (local)") by itself.
- An update removes the Start menu folder of the old name: one folder "Lang Switcher" with two shortcuts and one entry in
  Apps are left.
- Uninstalling no longer leaves the log behind: `diag.log` in `%APPDATA%\Lang_Switcher` is always deleted. "Yes" to
  "Remove the Lang Switcher settings file as well?" removes the settings, their backup copy and the reports saved by the
  "Write to the author" wizard — and the whole folder; "No" keeps the settings, as before.
- What did not change: the hotkey, the conversion, the windows and settings of the program, the settings file (schema 8),
  the author's signature.

## 0.100.0 — 2026-10-15

*A program can be excluded just by switching to it.*

- In the settings window, under "Exclusions", a program can now be added automatically: click "Pick a program window" and
  switch to the program within 5 seconds — the name of its process goes into the list and is selected, and "Added: …"
  appears under the button. The countdown runs on the button itself; a second click cancels it. If the program is listed
  already, it says "Already in the list: …"; if you did not switch anywhere, "No window picked".
- The old way — a process name in the field and "Add" — is still there, now under "Manually". Exclusions take effect, as
  before, on "OK" or "Apply"; "Cancel" drops the automatically added ones too.
- In the list of exclusions a click marks a row or takes the mark off — several can be marked and removed with one click
  on "Remove".
- Microsoft Store programs (Calculator, Settings) are shown by Windows in a shared frame, `ApplicationFrameHost.exe`. Now both
  picking a window and checking an exclusion take the program's own process (for example, `CalculatorApp.exe`), so
  excluding one such program does not exclude the others. The "Write to the author" wizard names the program's own process
  too.
- Only the file name of the process is taken; the window title — which may carry the name of a document or a message — is
  never read.
- What did not change: the hotkey, the conversion, the other sections of the settings, the settings file (schema 8), the
  signature and everything the installer does with the certificate.

## 0.99.0 — 2026-10-13

*The installer in the style of the site.*

- The setup wizard wears the style of the author's site, panda-pishet-kod.dev: the background in the colour of the site,
  with bamboo at the left and the right edge, as on the margins of the site's pages. The wizard follows the theme of
  Windows: light in the light theme, dark in the dark one (before, it was always light).
- At the bottom of every page of the wizard is the author's signature: the logo, "Panda Koder · programs made with heart"
  and the address of the site. A click on the address opens the site in your browser, without the administrator rights the
  installer runs with; the installer itself never goes to the network.
- On the first page, "Ready to Install", the line above the field no longer mentions a Back button that is not there.
- The windows of uninstalling are of the same colour as the wizard, without the bamboo and the signature.
- What did not change: the program itself — its windows, the hotkey and its settings, the settings file (schema 8), the
  signature and everything the installer does with the certificate.

## 0.98.0 — 2026-10-13

*The installer shows the sentence about the certificate whole.*

- The installer: on its first page, "Ready to Install", the sentence about the certificate the installer will make on your
  computer now wraps into lines and is seen whole. Before, it ran as one line off the right edge of the window.
- The description of the program on the page of the repository (README) has a section on what to do when the browser or
  Windows warns you — the same path as in the text of the release.
- The program chooses the font size of its windows by the screen, so that its tallest window fits whole. That count now
  includes the "From the author" window with the feed live: the line about reading the feed with its date, the "Recent
  letters" button and the feed switch that appears after 90 days. On the screens checked — 1920 × 1080 at 100, 125 and
  150 %, 1366 × 768, 2560 × 1440, 3840 × 2160 — the font size is the same as before; you will not see a difference.
- What did not change: the hotkey and its settings, the settings file (schema 8), the signature and everything the
  installer does with the certificate.

## 0.97.0 — 2026-10-08

*The double Shift answers faster.*

- When the hotkey is the double press of `Shift`, the program now asks Windows about three keys only on every tap of
  `Shift` — `Ctrl`, `Alt` and `Win` — instead of every modifier at once. That is enough to tell a tap from `Alt+Shift` or
  `Ctrl+Shift`, and the answer is exactly the one it was. Handling a press of `Shift` got shorter and fits the program's
  response-time limit again — 100 microseconds for 99 presses out of 100 (on the author's bench it was 168, now 94…99).
  You will not feel the difference by hand: it is headroom, not a new feature.
- What did not change: the rule of the double press — two short taps in a row within 400 ms, nothing between them —, the
  combinations with `Ctrl`, `Alt`, `Shift`, the default `Pause` key, the settings file (schema 8), the installer and the
  signature are the same.

## 0.96.0 — 2026-10-08

*The word no longer vanishes in the classic console.*

- In the classic console of Windows — the `conhost` window that `cmd` and Windows PowerShell open in when Windows Terminal
  is not the default terminal — the word no longer vanishes after the hotkey. In Windows PowerShell 5.1 the program erased
  the word and the console did not take the replacement. Now the program switches the window's layout first, then erases
  the word and presses the same keys you pressed: `ghbdtn` → `привет`, once more and `ghbdtn` is back; capitals too.
- If what you typed cannot be repeated with keys — it has a dead key or `AltGr` (found in layouts with diacritics) — the
  word in the classic console stays as typed and the refusal sound plays: nothing is lost.
- What did not change: Windows Terminal, Notepad, browsers and every other window behave as before; selected text in
  consoles is still not converted; a replacement method set by hand in the settings file (`[replacement] method =
  "backspace"` or `"selection"`) works without the automatic choice, and with it the classic console behaves as in 0.95.0;
  the settings file (schema 8), the installer and the signature are the same.

## 0.95.0 — 2026-10-07

*A double press of Shift.*

- The hotkey can be a **double press of `Shift`**: in the settings, click "Set" and press `Shift` twice in a row; the field
  shows "Shift, Shift". Two short taps of `Shift` convert the word, two more bring it back. Handy when the keyboard has no
  `Pause` and you would rather not reach for a combination.
- A tap is `Shift` pressed and released within 0.4 s, and the second tap begins within 0.4 s of the first. Ordinary typing
  with `Shift` is not affected: capital letters, `Alt+Shift` (the layout switch) and `Shift` with a mouse click (selection)
  are not taps.
- The program does not swallow `Shift` — other programs see it too. A program with its own command on a double `Shift`
  (Search Everywhere in JetBrains IDEs) will run it as well.
- Windows treats five presses of `Shift` in a row as its own shortcut and offers to turn on Sticky Keys. While the double
  `Shift` is the hotkey, the program turns that shortcut off — until it exits, nothing is saved in the settings of
  Windows; choosing another key, exiting or uninstalling brings it back. Sticky Keys turned on are left alone.
- Only `Shift` can be pressed twice; `Ctrl` and `Alt` go only with a key, as in 0.93.0.
- The settings file has a new field, `[hotkey] trigger`, and schema stamp 8. If you go back to 0.94 or older, the old
  version will not rewrite the file, but it will not understand the double press either and will answer to `Pause`.

## 0.94.0 — 2026-10-07

*A letter without the frame on a button.*

- A letter the program opens by itself (the welcome after the first start, a letter in a quiet minute) no longer shows the
  dotted frame on the "Open settings" button, as if it had been chosen in advance. The frame appears after the first
  `Tab`, as in a window opened with the mouse. `Enter` still presses "Got it", `Space` the button holding the focus.
- What did not change: the windows you open yourself (the tray menu, a click on the notification) behave as before; the
  look of the windows, the texts, the settings file (schema 7), the installer and the signature are the same.

## 0.93.0 — 2026-10-07

*A combination as the hotkey.*

- The hotkey can be a **combination**: a key together with `Ctrl`, `Alt`, `Shift` in any set — `Ctrl+F12`, `Alt+Shift+K`.
  In the settings, click "Set" and press the combination; the line under the field says what can be set. Handy when the
  keyboard has no `Pause`.
- A combination fires only on the **exact** set: with `Ctrl+F12`, a bare `F12` and `Ctrl+Shift+F12` stay with your
  programs. The default `Pause` works as before, a held `Shift` included.
- The capture does not take a key that types a character — alone, with `Shift` or with `AltGr` (`Ctrl+Alt`): the hotkey
  would take that character away from typing in every program. `Ctrl+K`, `Alt+K` are fine.
- A combination with `Alt`: so that releasing `Alt` does not open the window's menu, the program sends the window a press
  of an unassigned key the moment the hotkey fires — the technique of AutoHotkey. Without it, replacing selected text with
  `Alt` held went into the menu.
- The settings file has a new field, `[hotkey] modifiers`, and schema stamp 7. If you go back to 0.92 or older, the old
  version will not rewrite the file, but it reads a combination as its bare key: `Ctrl+F12` becomes `F12`.

## 0.92.0 — 2026-10-07

*Two findings of the author.*

- The welcome letter shown at the first start: the "ghbdtn → привет" demonstration no longer flickers. It used to be drawn
  straight into the window in several strokes, and on a slow machine a frame without the word showed between them; now
  the frame is assembled in a buffer and shown whole (on the author's screen, 63 transitional frames out of 780 became zero).
- Consoles, stated honestly: in Windows Terminal and the classic console the **typed** word is converted ("type +
  `Pause`"), while **selected** text, by mouse or `Shift`+arrows, is not. The only way to learn about a selection is to send
  `Ctrl+C`, and without a selection that cancels the typed line and interrupts a running command. The program's
  description now says so; the code did not change.

## 0.91.0 — 2026-10-07

*The delivery repaired.*

- The installer installs the **full** version — the one that also works in windows of programs run as administrator. To do
  so it creates a certificate on your computer, "Lang Switcher for Windows (local)": for code signing only, not a
  certification authority; signs the program with it and **destroys the private key at once** — nothing else can ever be
  signed with that certificate. It lies in the computer's Trusted Root Certification Authorities, with a copy in the
  program folder; **uninstalling the program removes it**. The installer does not use the network.
- If the certificate cannot be created, the **base** version is installed — the same code without `uiAccess`: it works
  everywhere except in windows of programs run as administrator. `LangSwitcher-setup.exe /NOLOCALCERT` installs it too.
- After installation the program starts without an error window.
- The uninstaller closes a running program by itself, the way Windows does at sign-out; only if it has not closed within
  10 seconds does it ask you to exit it (the tray icon's menu → Exit), and Cancel cancels the uninstall entirely.
- The setup wizard speaks the language of Windows (the program's thirteen languages; Greek Windows gets English).
- The author is named "Panda Koder".

## 0.90.0 — 2026-10-03

*The first public build.*

- A fourteen-language interface; the Settings, About and "From the author" windows; the theme follows the system;
  per-program exclusions; no typing buffer in password fields; the emergency exit `Ctrl+Alt+Shift+F12`.
- This build ran only on the author's machine. On any other Windows the program did not start — "A referral was returned
  from the server" (error 8235) — and the installer ended with "CreateProcess failed; code 740". Both are fixed in 0.91.0.
