# 功能：测量本仓库 PinPage 及其子进程的私有字节、工作集和短时 CPU。
param(
    [ValidateSet(1, 3, 5)] [int]$WindowCount = 1,
    [ValidateRange(2, 30)] [int]$Seconds = 5,
    [string]$Label = 'timer-idle'
)
$ErrorActionPreference = 'Stop'
$taskWorkspace = Split-Path -Parent $PSScriptRoot
$taskApplicationPaths = @(
    (Join-Path $taskWorkspace 'src-tauri\target\release\pinpage.exe'),
    (Join-Path $taskWorkspace 'src-tauri\target\debug\pinpage.exe')
)
$taskProcesses = @(Get-CimInstance Win32_Process)
$taskRootProcesses = @($taskProcesses | Where-Object { $_.ExecutablePath -in $taskApplicationPaths })
if ($taskRootProcesses.Count -ne 1) { throw '请只运行一个本仓库的 PinPage 实例后重试。' }
$taskRoot = $taskRootProcesses[0]
$taskProcessIds = [System.Collections.Generic.HashSet[int]]::new()
[void]$taskProcessIds.Add([int]$taskRoot.ProcessId)
do {
    $taskAdded = $false
    foreach ($taskProcess in $taskProcesses) {
        if ($taskProcessIds.Contains([int]$taskProcess.ParentProcessId)) {
            if ($taskProcessIds.Add([int]$taskProcess.ProcessId)) { $taskAdded = $true }
        }
    }
} while ($taskAdded)
$taskCpuBefore = @{}
foreach ($taskProcessId in $taskProcessIds) {
    $taskProcess = Get-Process -Id $taskProcessId -ErrorAction SilentlyContinue
    if ($taskProcess) { $taskCpuBefore[$taskProcessId] = $taskProcess.CPU }
}
Start-Sleep -Seconds $Seconds
$taskPrivateBytes = 0L
$taskWorkingSet = 0L
$taskCpuDelta = 0.0
$taskAliveCount = 0
foreach ($taskProcessId in $taskProcessIds) {
    $taskProcess = Get-Process -Id $taskProcessId -ErrorAction SilentlyContinue
    if (-not $taskProcess) { continue }
    $taskPrivateBytes += $taskProcess.PrivateMemorySize64
    $taskWorkingSet += $taskProcess.WorkingSet64
    if ($taskCpuBefore.ContainsKey($taskProcessId)) { $taskCpuDelta += [Math]::Max([double]0, [double]($taskProcess.CPU - $taskCpuBefore[$taskProcessId])) }
    $taskAliveCount++
}
$taskRuntime = $taskProcesses | Where-Object { $taskProcessIds.Contains([int]$_.ProcessId) -and $_.Name -eq 'msedgewebview2.exe' } | Select-Object -First 1
$taskRuntimeVersion = if ($taskRuntime.ExecutablePath) { (Get-Item -LiteralPath $taskRuntime.ExecutablePath).VersionInfo.FileVersion } else { 'unknown' }
$taskResult = [PSCustomObject]@{
    timestamp = (Get-Date -Format o)
    label = $Label
    windowCount = $WindowCount
    build = if ($taskRoot.ExecutablePath -like '*\release\*') { 'Release' } else { 'Debug' }
    sampleSeconds = $Seconds
    processCount = $taskAliveCount
    privateMiB = [Math]::Round($taskPrivateBytes / 1MB, 2)
    rawWorkingSetMiB = [Math]::Round($taskWorkingSet / 1MB, 2)
    cpuPercentOfOneCore = [Math]::Round($taskCpuDelta / $Seconds * 100, 2)
    webview2Version = $taskRuntimeVersion
    os = [Environment]::OSVersion.VersionString
    logicalProcessors = [Environment]::ProcessorCount
    note = '窗口数由调用方确认；私有字节不是私有驻留内存；工作集简单求和包含共享页，不能作为实际物理内存总量。短时样本不代表长期表现。'
}
$taskOutputDirectory = Join-Path $taskWorkspace 'runtime-data'
New-Item -ItemType Directory -Path $taskOutputDirectory -Force | Out-Null
$taskOutputPath = Join-Path $taskOutputDirectory 'memory-samples.jsonl'
$taskResult | ConvertTo-Json -Compress | Add-Content -LiteralPath $taskOutputPath -Encoding UTF8
$taskResult | ConvertTo-Json
