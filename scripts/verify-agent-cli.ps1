# 功能：在显式独立实例验证随包 CLI 的页面、诊断、截图和 1／3／5 页资源闭环，保留源码与报告。
param(
    [Parameter(Mandatory)] [string]$CliPath,
    [Parameter(Mandatory)] [string]$AppPath,
    [Parameter(Mandatory)] [string]$Instance,
    [string]$OutputPath = 'runtime-data/agent-cli/acceptance',
    [switch]$AllowCaptureUnavailable
)
$ErrorActionPreference = 'Stop'
if ($Instance -eq 'com.pinpage.desktop') { throw '验收脚本必须使用独立实例标识' }
$CliPath = (Resolve-Path -LiteralPath $CliPath).Path
$AppPath = (Resolve-Path -LiteralPath $AppPath).Path
$pinpageRun = [Guid]::NewGuid().ToString('N')
$pinpageOutput = Join-Path ([IO.Path]::GetFullPath($OutputPath)) $pinpageRun
$pinpageSite = Join-Path $pinpageOutput '中文 site'
New-Item -ItemType Directory -Path $pinpageSite -Force | Out-Null
$pinpageIds = [Collections.Generic.List[string]]::new()
$pinpageSessions = [Collections.Generic.List[string]]::new()
$pinpageReport = [ordered]@{ instance = $Instance; source = $pinpageSite; screenshots = @(); samples = @() }

# 职责：每条命令解析唯一 JSON 并检查退出码，冲突用例显式声明预期码。
function Invoke-PinPage {
    param([string[]]$Arguments, [int[]]$ExpectedCode = @(0))
    $pinpageCommand = @('--json', '--instance', $Instance) + $Arguments
    $pinpageRaw = & $CliPath @pinpageCommand
    $pinpageCode = $LASTEXITCODE
    $pinpageResponse = ($pinpageRaw -join "`n") | ConvertFrom-Json
    if ($ExpectedCode -notcontains $pinpageCode) { throw "CLI 退出码 $pinpageCode，预期 $ExpectedCode：$($pinpageResponse.error.message)" }
    return $pinpageResponse
}

@'
<!-- 功能：CLI 验收计时器，透明面板使用宿主变量。 -->
<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><link rel="stylesheet" href="style.css"><script src="app.js" defer></script><title>CLI 验收计时器</title></head><body><main><h1>专注计时</h1><p id="clock"></p><p id="state"></p></main></body></html>
'@ | Set-Content -LiteralPath (Join-Path $pinpageSite 'index.html') -Encoding utf8
@'
/* 功能：区分宿主透明开关和局部半透明面板，适配窄窗口。 */
:root{font-family:system-ui;color:#17233a;background:rgb(240 245 255 / var(--pinpage-background-opacity,1))}body{margin:0;padding:16px}main{border-radius:16px;background:rgb(255 255 255 / .72);padding:16px;text-align:center}h1{font-size:20px}#clock{font-size:40px;margin:8px 0}#state{font-size:12px}@media(max-width:280px){body{padding:4px}#clock{font-size:28px}}
'@ | Set-Content -LiteralPath (Join-Path $pinpageSite 'style.css') -Encoding utf8
@'
// 功能：校验 localStorage、sessionStorage、IndexedDB 的来源延续，制造可定位的异常。
const key='pinpage-cli-acceptance-v1';
const previous=JSON.parse(localStorage.getItem(key)||'null');
const state=previous||{deadline:Date.now()+1500000,identity:Date.now().toString(16)+Math.random().toString(16).slice(2)};
localStorage.setItem(key,JSON.stringify(state));
console.log('CLI_TEST_V1',JSON.stringify(state),'restored='+Boolean(previous));
const visits=Number(sessionStorage.getItem(key)||0)+1;sessionStorage.setItem(key,String(visits));console.log('SESSION_VISITS',visits);
// 职责：按截止时间校正计时，刷新和关闭重开使用同一截止时间。
function tick(){const left=Math.max(0,Math.ceil((state.deadline-Date.now())/1000));document.querySelector('#clock').textContent=Math.floor(left/60)+':'+String(left%60).padStart(2,'0');document.querySelector('#state').textContent=previous?'已恢复保存状态':'首次保存状态';}
tick();setInterval(tick,1000);
const request=indexedDB.open('pinpage-cli-acceptance',1);
request.onupgradeneeded=()=>request.result.createObjectStore('state');
request.onsuccess=()=>{const db=request.result;const tx=db.transaction('state','readwrite');const store=tx.objectStore('state');const get=store.get('identity');get.onsuccess=()=>{const identity=get.result||state.identity;if(!get.result)store.put(identity,'identity');console.log('IDB_IDENTITY',identity,'restored='+Boolean(get.result));};tx.oncomplete=()=>db.close();};
fetch('missing.json?credential=must-be-redacted').catch(()=>{});
setTimeout(()=>{throw new Error('CLI_TEST_EXPECTED_ERROR');},100);
'@ | Set-Content -LiteralPath (Join-Path $pinpageSite 'app.js') -Encoding utf8

try {
    $null = Invoke-PinPage @('app', 'start', '--app-path', $AppPath)
    $created = Invoke-PinPage @('pages', 'install', '--source', $pinpageSite, '--entry', 'index.html', '--client-key', $pinpageRun)
    $pinpageId = $created.data.pageId
    $pinpageIds.Add($pinpageId)
    $duplicate = Invoke-PinPage @('pages', 'install', '--source', $pinpageSite, '--entry', 'index.html', '--client-key', $pinpageRun)
    if ($duplicate.data.pageId -ne $pinpageId -or -not $duplicate.data.deduplicated) { throw '安装去重失败' }
    $debug = Invoke-PinPage @('debug', 'start', $pinpageId, '--reload')
    $pinpageSessions.Add($debug.data.sessionId)
    $null = Invoke-PinPage @('pages', 'wait', $pinpageId, '--load-id', $debug.data.loadId, '--settle-ms', '300')
    $before = Invoke-PinPage @('debug', 'logs', $debug.data.sessionId, '--limit', '100')
    if (-not ($before.data.records | Where-Object { $_.message -like '*CLI_TEST_EXPECTED_ERROR*' })) { throw '未捕获故意脚本异常' }
    if (($before.data.records | Where-Object source -eq 'resource' | ConvertTo-Json) -match 'must-be-redacted') { throw '资源 URL 未脱敏' }
    foreach ($frame in @('system', 'borderless', 'rounded')) {
        $null = Invoke-PinPage @('window', 'set', $pinpageId, '--width', '400', '--height', '260', '--frame', $frame, '--transparent', $(if ($frame -eq 'rounded') { 'true' } else { 'false' }))
        $path = Join-Path $pinpageOutput "$frame.png"
        if (-not $pinpageReport.captureFailure) {
            $capture = Invoke-PinPage -Arguments @('screenshot', $pinpageId, '--out', $path) -ExpectedCode $(if ($AllowCaptureUnavailable) { @(0,7) } else { @(0) })
            if ($capture.ok) { $pinpageReport.screenshots += $capture.data } else { $pinpageReport.captureFailure = $capture.error }
        }
    }
    $current = Invoke-PinPage @('pages', 'inspect', $pinpageId)
    $pinpageScript = Join-Path $pinpageSite 'app.js'
    (Get-Content -LiteralPath $pinpageScript -Raw).Replace('CLI_TEST_V1', 'CLI_TEST_V2') | Set-Content -LiteralPath $pinpageScript -Encoding utf8
    $updated = Invoke-PinPage @('pages', 'update', $pinpageId, '--source', $pinpageSite, '--entry', 'index.html', '--expected-revision', "$($current.data.revision)", '--expected-instance', $current.instanceId)
    $null = Invoke-PinPage @('pages','wait',$pinpageId,'--load-id',$updated.data.loadId,'--settle-ms','100')
    $null = Invoke-PinPage -Arguments @('pages', 'update', $pinpageId, '--source', $pinpageSite, '--entry', 'index.html', '--expected-revision', "$($current.data.revision)", '--expected-instance', $current.instanceId) -ExpectedCode 5
    $after = Invoke-PinPage @('debug', 'logs', $debug.data.sessionId, '--cursor', "$($before.data.nextCursor)", '--limit', '100')
    $oldState = ($before.data.records | Where-Object message -Like 'CLI_TEST_V1 *' | Select-Object -Last 1).message
    $newState = ($after.data.records | Where-Object message -Like 'CLI_TEST_V2 *' | Select-Object -Last 1).message
    if (-not $oldState -or $oldState.Replace('CLI_TEST_V1','CLI_TEST_V2') -ne $newState) { throw '更新未保留计时状态' }
    $oldDatabase = ($before.data.records | Where-Object message -Like 'IDB_IDENTITY *' | Select-Object -Last 1).message
    $newDatabase = ($after.data.records | Where-Object message -Like 'IDB_IDENTITY *' | Select-Object -Last 1).message
    if (-not $oldDatabase -or $oldDatabase -ne $newDatabase) { throw '更新未保留 IndexedDB' }
    $null = Invoke-PinPage @('debug', 'stop', $debug.data.sessionId)
    $pinpageSessions.Clear()
    $null = Invoke-PinPage @('pages', 'close', $pinpageId)
    $closed = Invoke-PinPage @('pages', 'inspect', $pinpageId)
    $closedUpdate = Invoke-PinPage @('pages', 'update', $pinpageId, '--source', $pinpageSite, '--entry', 'index.html', '--expected-revision', "$($closed.data.revision)", '--expected-instance', $closed.instanceId)
    if ($closedUpdate.data.loadState -ne 'notRequested') { throw '关闭页更新被意外开启' }
    $null = Invoke-PinPage @('pages', 'open', $pinpageId)
    $reopened = Invoke-PinPage @('debug','start',$pinpageId,'--reload')
    $pinpageSessions.Add($reopened.data.sessionId)
    $null = Invoke-PinPage @('pages','wait',$pinpageId,'--load-id',$reopened.data.loadId,'--settle-ms','200')
    $reopenedLogs = Invoke-PinPage @('debug','logs',$reopened.data.sessionId,'--limit','100')
    if (($reopenedLogs.data.records | Where-Object message -Like 'CLI_TEST_V2 *' | Select-Object -Last 1).message -ne $newState) { throw '关闭重开未保留 localStorage' }
    if (($reopenedLogs.data.records | Where-Object message -Like 'IDB_IDENTITY *' | Select-Object -Last 1).message -ne $newDatabase) { throw '关闭重开未保留 IndexedDB' }
    if (-not ($reopenedLogs.data.records | Where-Object message -EQ 'SESSION_VISITS 2')) { throw '新 WebView 会话未重置 sessionStorage' }
    $null=Invoke-PinPage @('debug','stop',$reopened.data.sessionId)
    $pinpageSessions.Clear()
    $pinpageReport.pageId = $pinpageId
    $pinpageReport.revision = $closedUpdate.data.revision
    $pinpageReport.storageBefore = $oldState
    $pinpageReport.storageAfter = $newState
    $pinpageReport.indexedDb = $newDatabase
    $pinpageReport.sessionStorageReset = $true
    for ($i=2; $i -le 5; $i++) {
        $page = Invoke-PinPage @('pages', 'install', '--source', $pinpageSite, '--entry', 'index.html', '--closed', '--client-key', "$pinpageRun-$i")
        $pinpageIds.Add($page.data.pageId)
    }
    foreach ($count in @(1,3,5)) {
        for ($i=1; $i -lt $count; $i++) { $null = Invoke-PinPage @('pages','open',$pinpageIds[$i]) }
        $pinpageReport.samples += (& (Join-Path $PSScriptRoot 'measure-memory.ps1') -AppPath $AppPath -BuildMode Release -WindowCount $count -Seconds 2 -Label "cli-idle-$count" | ConvertFrom-Json)
    }
    foreach ($pageId in $pinpageIds) { $session=Invoke-PinPage @('debug','start',$pageId,'--reload');$pinpageSessions.Add($session.data.sessionId) }
    $pinpageReport.samples += (& (Join-Path $PSScriptRoot 'measure-memory.ps1') -AppPath $AppPath -BuildMode Release -WindowCount 5 -Seconds 2 -Label 'cli-diagnostics-5' | ConvertFrom-Json)
    $pinpageReport.functionalChecksPassed = $true
    $pinpageReport.ok = -not $pinpageReport.captureFailure
} finally {
    foreach ($session in $pinpageSessions) { $null = Invoke-PinPage @('debug','stop',$session) }
    foreach ($pageId in $pinpageIds) { $null = Invoke-PinPage @('pages','close',$pageId) }
    $pinpageReport | ConvertTo-Json -Depth 16 | Set-Content -LiteralPath (Join-Path $pinpageOutput 'report.json') -Encoding utf8
}
[pscustomobject]@{ report = (Join-Path $pinpageOutput 'report.json'); pageId = $pinpageReport.pageId; ok = $pinpageReport.ok } | ConvertTo-Json
