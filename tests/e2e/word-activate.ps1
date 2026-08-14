<#
    word-activate.ps1 — задача T-04-3, стенд приёмки.

    Выводит окно процесса вперёд через WScript.Shell.AppActivate.

    ЗАЧЕМ ОТДЕЛЬНЫЙ СКРИПТ. Грабля 3 §2 TOOLCHAIN.md: SetForegroundWindow из фонового
    процесса Windows игнорирует, а AutomationElement.SetFocus на документе Word бросает
    "Target element cannot receive focus". Рабочий способ — AppActivate по идентификатору
    процесса; после него фокус встаёт на _WwG и SendInput доходит. AppActivate — метод
    позднего связывания через IDispatch, и решение Р-29 прямо разрешает вызывать его
    дочерним процессом PowerShell, а не переписывать на Rust.

    ОДНА ПОПЫТКА, БЕЗ ЦИКЛА И БЕЗ Start-Sleep. Повтором управляет стенд через wait::until —
    единственное место во всём tests\e2e\, где процесс спит. Скрипт, который ждал бы сам,
    завёл бы вторую задержку вне этого места и сделал бы требование 1 §11.5 непроверяемым
    сплошным поиском.

    Коды возврата: 0 — окно выведено, 1 — не выведено, 2 — ошибка вызова.
#>
param(
    [Parameter(Mandatory = $true)]
    [int]$ProcessId
)

$ErrorActionPreference = 'Stop'

try {
    $shell = New-Object -ComObject WScript.Shell
    if ($shell.AppActivate($ProcessId)) {
        Write-Output 'activated'
        exit 0
    }
    Write-Output 'not-activated'
    exit 1
}
catch {
    Write-Output ('error: ' + $_.Exception.Message)
    exit 2
}
