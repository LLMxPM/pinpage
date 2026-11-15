# 功能：在独立实例验证窗口预设管理、页面包往返、冲突保留和真实桌面截图。
param(
    [Parameter(Mandatory)] [string]$CliPath,
    [Parameter(Mandatory)] [string]$AppPath,
    [Parameter(Mandatory)] [string]$Instance,
    [string]$OutputPath = 'runtime-data/window-presets',
    [switch]$AllowCaptureUnavailable
)
$ErrorActionPreference = 'Stop'
if ($Instance -in @('com.pinpage.desktop', 'com.pinpage.development')) { throw '必须使用独立验证实例' }
$CliPath = (Resolve-Path -LiteralPath $CliPath).Path
$AppPath = (Resolve-Path -LiteralPath $AppPath).Path
$pinpageRun = [Guid]::NewGuid().ToString('N')
$pinpageOutput = Join-Path ([IO.Path]::GetFullPath($OutputPath)) $pinpageRun
$pinpageSite = Join-Path $pinpageOutput '中文 site'
New-Item -ItemType Directory -Path $pinpageSite -Force | Out-Null
$pinpageIds = [Collections.Generic.List[string]]::new()
$pinpageChecks = 0
$pinpageReport = [ordered]@{ instance = $Instance; checks = 0; screenshots = @(); ok = $false }

# 职责：检查唯一 JSON、退出码与成功标记，预期失败用例明确声明退出码。
function Invoke-PinPage {
    param([string[]]$Arguments, [int[]]$ExpectedCode = @(0))
    $pinpageRaw = & $CliPath --json --instance $Instance @Arguments
    $pinpageCode = $LASTEXITCODE
    $pinpageResponse = ($pinpageRaw -join "`n") | ConvertFrom-Json
    if ($ExpectedCode -notcontains $pinpageCode -or $pinpageResponse.ok -ne ($pinpageCode -eq 0)) {
        throw "退出码 $pinpageCode，预期 $ExpectedCode：$($pinpageResponse | ConvertTo-Json -Depth 8 -Compress)"
    }
    return $pinpageResponse
}

# 职责：累计业务断言，失败立即停止，最终仍关闭本脚本创建的页面。
function Assert-PinPage {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
    $script:pinpageChecks++
}

# 职责：变更前读取最新修订和实例；只向已经保存的稳定页面 ID 提交。
function Set-Preset {
    param([string]$Page, [string[]]$Action)
    $current = Invoke-PinPage @('pages', 'inspect', $Page)
    return Invoke-PinPage (@('window', 'presets') + $Action + @('--expected-revision', "$($current.data.revision)", '--expected-instance', $current.instanceId))
}

# 职责：读取页面包的实际 JSON 条目，不依赖导出响应来推断包内数据完整性。
function Read-PackageJson {
    param([string]$Path, [string]$Entry = 'manifest.json')
    $archive = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        $item = $archive.GetEntry($Entry)
        if (-not $item) { return $null }
        $reader = [IO.StreamReader]::new($item.Open())
        try { return $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
    } finally { $archive.Dispose() }
}

@'
<!-- 功能：预设验收静态页面，尺寸与透明度跟随原生窗口。 -->
<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><link rel="stylesheet" href="style.css"><script src="app.js" defer></script><title>预设验收</title></head><body><main><h1>窗口预设验收</h1><p>精简与展开布局</p><output id="size"></output></main></body></html>
'@ | Set-Content -LiteralPath (Join-Path $pinpageSite 'index.html') -Encoding utf8
@'
/* 功能：透明圆角面板在不同窗口大小下保持内容可见。 */
html{background:rgb(245 247 251 / var(--pinpage-background-opacity,1));font-family:system-ui;color:#162942}body{margin:0;padding:16px}main{padding:20px;border-radius:16px;background:rgb(255 255 255 / .8);box-shadow:0 8px 30px #22334420}h1{font-size:22px}output{font-size:14px}
'@ | Set-Content -LiteralPath (Join-Path $pinpageSite 'style.css') -Encoding utf8
@'
// 功能：记录原始页面状态，验证预设应用没有重新导航。
localStorage.setItem('pinpage-presets-test','保存状态');
/// 窗口缩放时更新视区尺寸。
function resize(){document.querySelector('#size').textContent=innerWidth+' × '+innerHeight;}
addEventListener('resize',resize);resize();
'@ | Set-Content -LiteralPath (Join-Path $pinpageSite 'app.js') -Encoding utf8
$template = [ordered]@{ name = '精简'; mode = 'floating'; alwaysOnTop = $true; decorations = $false; rounded = $true; transparentBackground = $true; positionLocked = $false; sizeLocked = $false; width = 320; height = 220; fx = 1; fy = 0; fullscreen = $false }
$compact = Join-Path $pinpageOutput 'compact.json'
$template | ConvertTo-Json | Set-Content -LiteralPath $compact -Encoding utf8
$template.name = '展开'; $template.width = 640; $template.height = 420; $template.fx = 0.5; $template.fy = 0.5; $template.transparentBackground = $false
$expanded = Join-Path $pinpageOutput 'expanded.json'
$template | ConvertTo-Json | Set-Content -LiteralPath $expanded -Encoding utf8

try {
    $null = Invoke-PinPage @('app', 'start', '--app-path', $AppPath)
    $doctor = Invoke-PinPage @('doctor')
    Assert-PinPage ($doctor.data.capabilities.windowPresets -eq 1 -and $doctor.data.capabilities.pageExport -eq 1) '缺少新增能力'
    $created = Invoke-PinPage @('pages', 'install', '--source', $pinpageSite, '--closed', '--client-key', $pinpageRun)
    $page = $created.data.pageId; $pinpageIds.Add($page)
    $before = Invoke-PinPage @('pages', 'inspect', $page)
    $null = Set-Preset $page @('put', $page, '--id', 'compact', '--file', $compact)
    $null = Set-Preset $page @('put', $page, '--id', 'compact', '--file', $compact)
    $listed = Invoke-PinPage @('window', 'presets', 'list', $page)
    Assert-PinPage ($listed.data.presets.Count -eq 1) '稳定 ID 重试增加了重复预设'
    $null = Invoke-PinPage -Arguments @('window', 'presets', 'apply', $page, 'compact', '--expected-revision', "$($before.data.revision)", '--expected-instance', $before.instanceId) -ExpectedCode 5
    $null = Set-Preset $page @('put', $page, '--id', 'expanded', '--file', $expanded)
    $after = Invoke-PinPage @('pages', 'inspect', $page)
    Assert-PinPage (-not $after.data.config.enabled -and $after.data.config.width -eq $before.data.config.width -and $after.data.config.height -eq $before.data.config.height) '编辑预设改变了窗口或开关'
    $invalid = Join-Path $pinpageOutput 'invalid.json'
    $invalidTemplate = Get-Content -LiteralPath $expanded -Raw | ConvertFrom-Json -AsHashtable
    $invalidTemplate.folderRoot = 'forbidden'
    $invalidTemplate | ConvertTo-Json | Set-Content -LiteralPath $invalid -Encoding utf8
    $null = Invoke-PinPage -Arguments @('window', 'presets', 'put', $page, '--id', 'invalid', '--file', $invalid, '--expected-revision', "$($after.data.revision)", '--expected-instance', $after.instanceId) -ExpectedCode 2
    $null = Set-Preset $page @('save', $page, '--id', 'saved', '--name', '保存当前窗口')
    $saved = Invoke-PinPage @('window', 'presets', 'list', $page)
    Assert-PinPage ($saved.data.activePresetId -eq 'saved' -and $saved.data.presets.Count -eq 3) '捕获当前窗口未保存匹配标记'
    $null = Set-Preset $page @('delete', $page, 'saved')
    $null = Set-Preset $page @('apply', $page, 'compact')
    $closed = Invoke-PinPage @('pages', 'inspect', $page)
    Assert-PinPage (-not $closed.data.open -and -not $closed.data.config.enabled -and $closed.data.config.width -eq 320) '关闭页应用预设未正确保留关闭状态'
    $package = Join-Path $pinpageOutput '完整模板.pinpage'
    $exported = Invoke-PinPage @('pages', 'export', $page, '--out', $package)
    $manifest = Read-PackageJson $package
    Assert-PinPage ($exported.data.presetCount -eq 2 -and -not $exported.data.withData -and $manifest.windowPresets[0].id -eq 'compact' -and $manifest.windowPresets[1].id -eq 'expanded') '导出遗漏预设或顺序'
    $digest = (Get-FileHash -LiteralPath $package).Hash
    $null = Invoke-PinPage -Arguments @('pages', 'export', $page, '--out', $package) -ExpectedCode 8
    Assert-PinPage ((Get-FileHash -LiteralPath $package).Hash -eq $digest) '未授权覆盖改变了原包'
    $null = Invoke-PinPage @('pages', 'export', $page, '--out', $package, '--overwrite')
    $imported = Invoke-PinPage @('pages', 'install', '--source', $package, '--closed', '--apply-package-preset', 'false')
    $copy = $imported.data.pageId; $pinpageIds.Add($copy)
    $copyState = Invoke-PinPage @('pages', 'inspect', $copy)
    Assert-PinPage ($copyState.data.config.presets.Count -eq 2 -and $copyState.data.config.width -eq 420) '导入列表与应用默认布局未分离'
    $fields = @($manifest.windowPresets[0].PSObject.Properties.Name | Sort-Object)
    $actualPresets = $copyState.data.config.presets | Select-Object -Property $fields | ConvertTo-Json -Depth 8 -Compress
    $expectedPresets = $manifest.windowPresets | Select-Object -Property $fields | ConvertTo-Json -Depth 8 -Compress
    Assert-PinPage ($actualPresets -eq $expectedPresets) '导入未保留全部模板字段'
    $noList = Invoke-PinPage @('pages', 'install', '--source', $package, '--closed', '--import-package-presets', 'false')
    $pinpageIds.Add($noList.data.pageId)
    $noListState = Invoke-PinPage @('pages', 'inspect', $noList.data.pageId)
    Assert-PinPage ($noListState.data.config.presets.Count -eq 0 -and $noListState.data.config.width -eq 320) '取消导入列表同时取消了默认布局'
    $null = Set-Preset $copy @('put', $copy, '--id', 'compact', '--file', $expanded)
    foreach ($policy in @('keep', 'merge', 'replace')) {
        $current = Invoke-PinPage @('pages', 'inspect', $copy)
        $expected = if ($policy -eq 'merge') { 5 } else { 0 }
        $update = Invoke-PinPage -Arguments @('pages', 'update', $copy, '--source', $package, '--presets', $policy, '--expected-revision', "$($current.data.revision)", '--expected-instance', $current.instanceId) -ExpectedCode $expected
        $state = Invoke-PinPage @('pages', 'inspect', $copy)
        Assert-PinPage (-not $state.data.open -and $state.data.config.width -eq 420) "更新 $policy 改变了当前窗口或开关"
        if ($policy -eq 'keep') { Assert-PinPage ($state.data.config.presets[0].width -eq 640) '默认更新覆盖了用户模板' }
        if ($policy -eq 'merge') { Assert-PinPage ($update.error.code -eq 'presetConflict' -and $state.data.revision -eq $current.data.revision) '合并冲突未完整保留原配置' }
        if ($policy -eq 'replace') { Assert-PinPage ($state.data.config.presets[0].width -eq 320) '显式替换未更新模板' }
    }
    $null = Set-Preset $page @('put', $page, '--id', 'extra', '--file', $expanded)
    $null = Invoke-PinPage @('pages', 'export', $page, '--out', $package, '--overwrite')
    $current = Invoke-PinPage @('pages', 'inspect', $copy)
    $null = Invoke-PinPage @('pages', 'update', $copy, '--source', $package, '--presets', 'merge', '--expected-revision', "$($current.data.revision)", '--expected-instance', $current.instanceId)
    $merged = Invoke-PinPage @('window', 'presets', 'list', $copy)
    Assert-PinPage ($merged.data.presets.Count -eq 3 -and $merged.data.presets[2].id -eq 'extra') '显式合并没有追加新 ID 或保持顺序'
    $null = Invoke-PinPage @('pages', 'open', $copy)
    $loaded = Invoke-PinPage @('pages', 'inspect', $copy)
    foreach ($preset in @('compact', 'expanded')) {
        $null = Set-Preset $copy @('apply', $copy, $preset)
        $state = Invoke-PinPage @('pages', 'inspect', $copy)
        Assert-PinPage ($state.data.loadId -eq $loaded.data.loadId -and $state.data.config.enabled) '预设切换触发导航或关闭了页面'
        $path = Join-Path $pinpageOutput "$preset.png"
        $capture = Invoke-PinPage -Arguments @('screenshot', $copy, '--out', $path, '--bring-to-front') -ExpectedCode $(if ($AllowCaptureUnavailable) { @(0, 7) } else { @(0) })
        if ($capture.ok) { $pinpageReport.screenshots += $path } else { $pinpageReport.captureFailure = $capture.error }
    }
    $dataPackage = Join-Path $pinpageOutput '含数据.pinpage'
    $withData = Invoke-PinPage @('pages', 'export', $copy, '--out', $dataPackage, '--with-data', 'true')
    $storage = Read-PackageJson $dataPackage 'data/storage.json'
    Assert-PinPage ($withData.data.withData -and $storage.localStorage.'pinpage-presets-test' -eq '保存状态') '显式数据导出未携带运行页存储'
    $pinpageReport.ok = $true
} finally {
    foreach ($id in $pinpageIds) { $null = Invoke-PinPage @('pages', 'close', $id) }
    $pinpageReport.checks = $pinpageChecks
    $pinpageReport | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $pinpageOutput 'report.json') -Encoding utf8
}
[pscustomobject]@{ report = (Join-Path $pinpageOutput 'report.json'); ok = $pinpageReport.ok; checks = $pinpageChecks; screenshots = $pinpageReport.screenshots } | ConvertTo-Json
