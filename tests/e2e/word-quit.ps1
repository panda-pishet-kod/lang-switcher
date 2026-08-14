<#
    word-quit.ps1 — задача T-04-3, стенд приёмки.

    Закрывает Microsoft Word штатно, без сохранения.

    ЗАЧЕМ ИМЕННО ТАК. Две грабли §2 TOOLCHAIN.md сразу:

      • Грабля 4: WM_CLOSE Word игнорирует. Штатное закрытие — COM:
        GetActiveObject('Word.Application'), затем Quit(0), где 0 = wdDoNotSaveChanges.

      • Грабля 5: Stop-Process на Word отравляет СЛЕДУЮЩИЙ запуск — появляется модальный
        вопрос про безопасный режим и заводится запись в
        HKCU\...\Word\Resiliency\DisabledItems. Требование 5 §11.5 SPEC про восстановление
        состояния — условие работоспособности следующего сценария, а не формальность.
        Поэтому Stop-Process в этом скрипте НЕ вызывается ни при каком исходе.

    Оба вызова требуют позднего связывания через IDispatch и оба отлажены в probe-word.ps1;
    решение Р-29 разрешает выполнять их дочерним процессом PowerShell.

    Коды возврата: 0 — Word закрыт либо его и не было, 1 — Quit не сработала.
#>

$ErrorActionPreference = 'Stop'

try {
    $word = [Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application')
}
catch {
    # Экземпляра в таблице выполняющихся объектов нет — закрывать нечего.
    Write-Output 'no-instance'
    exit 0
}

try {
    # 0 = wdDoNotSaveChanges. Документы стенда содержат только его собственный маркёр,
    # и сохранять их некуда и незачем.
    $word.Quit(0)
    Write-Output 'quit'
    exit 0
}
catch {
    Write-Output ('quit-failed: ' + $_.Exception.Message)
    exit 1
}
