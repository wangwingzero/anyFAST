param(
  [string]$Domain = "betterclau.de",
  [string]$Path = "/claude/anyrouter.top",
  [string]$PreferredIp = "",
  [string]$PreferredIpList = "",
  [int]$IntervalMinutes = 15,
  [int]$TimeoutSec = 8,
  [int]$FailureThreshold = 2,
  [int]$BetterStreakToPin = 3,
  [int]$WorseStreakToUnpin = 2,
  [int]$CooldownMinutes = 30,
  [double]$BetterRatioThreshold = 0.30,
  [double]$BetterAbsoluteMs = 60
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$hostsPath = Join-Path $env:SystemRoot "System32\drivers\etc\hosts"
$stateDir = Join-Path $env:ProgramData "anyfast"
$statePath = Join-Path $stateDir "hosts_fallback_state.json"
$logPath = Join-Path $stateDir "hosts_fallback.log"
$startMark = "# <ANYFAST_HOSTS_START>"
$endMark = "# <ANYFAST_HOSTS_END>"

function Write-Log {
  param([string]$Message)
  if (-not (Test-Path $stateDir)) {
    New-Item -Path $stateDir -ItemType Directory -Force | Out-Null
  }
  $line = "{0} {1}" -f (Get-Date).ToString("yyyy-MM-dd HH:mm:ss"), $Message
  Add-Content -Path $logPath -Value $line -Encoding UTF8
}

function Ensure-StateField {
  param(
    [object]$State,
    [string]$Name,
    $Value
  )

  if (-not ($State.PSObject.Properties.Name -contains $Name)) {
    Add-Member -InputObject $State -MemberType NoteProperty -Name $Name -Value $Value
  }
}

function Test-IsAdmin {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = New-Object Security.Principal.WindowsPrincipal($identity)
  return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-State {
  if (Test-Path $statePath) {
    try {
      $state = (Get-Content $statePath -Raw | ConvertFrom-Json)
      Ensure-StateField -State $state -Name "mode" -Value "dns"
      Ensure-StateField -State $state -Name "betterStreak" -Value 0
      Ensure-StateField -State $state -Name "worseStreak" -Value 0
      Ensure-StateField -State $state -Name "pinnedFailStreak" -Value 0
      Ensure-StateField -State $state -Name "currentPinnedIp" -Value ""
      Ensure-StateField -State $state -Name "lastSwitchUtc" -Value ""
      Ensure-StateField -State $state -Name "lastCheckUtc" -Value ""
      return $state
    } catch {
      Write-Log "state parse failed, reset state. err=$($_.Exception.Message)"
    }
  }

  return [PSCustomObject]@{
    mode = "dns"
    betterStreak = 0
    worseStreak = 0
    pinnedFailStreak = 0
    currentPinnedIp = ""
    lastSwitchUtc = ""
    lastCheckUtc = ""
  }
}

function Save-State {
  param([object]$State)
  if (-not (Test-Path $stateDir)) {
    New-Item -Path $stateDir -ItemType Directory -Force | Out-Null
  }
  ($State | ConvertTo-Json -Depth 6) | Set-Content -Path $statePath -Encoding UTF8
}

function Remove-ManagedHostsBlock {
  $content = ""
  if (Test-Path $hostsPath) {
    $content = Get-Content $hostsPath -Raw
  }

  $pattern = "(?ms)\r?\n?$([regex]::Escape($startMark)).*?$([regex]::Escape($endMark))\r?\n?"
  $updated = [regex]::Replace($content, $pattern, "")
  Set-Content -Path $hostsPath -Value $updated -Encoding ASCII
}

function Set-ManagedHostsBlock {
  param([string]$EntryLine)

  $content = ""
  if (Test-Path $hostsPath) {
    $content = Get-Content $hostsPath -Raw
  }

  $pattern = "(?ms)\r?\n?$([regex]::Escape($startMark)).*?$([regex]::Escape($endMark))\r?\n?"
  $clean = [regex]::Replace($content, $pattern, "")
  $block = "`r`n$startMark`r`n$EntryLine`r`n$endMark`r`n"
  $next = ($clean.TrimEnd() + $block)
  Set-Content -Path $hostsPath -Value $next -Encoding ASCII
}

function Flush-DnsCache {
  & ipconfig /flushdns | Out-Null
}

function New-FailedRouteSample {
  return [PSCustomObject]@{
    ok = $false
    code = "000"
    ttfb = 999
    total = 999
  }
}

function Get-RouteSample {
  param(
    [string]$DomainName,
    [string]$RequestPath,
    [int]$Timeout,
    [string]$Ip = ""
  )

  $url = "https://$DomainName$RequestPath"
  $writeOut = "%{http_code}|%{time_starttransfer}|%{time_total}"
  $args = @(
    "--silent",
    "--show-error",
    "--output", "NUL",
    "--write-out", $writeOut,
    "--max-time", "$Timeout",
    "--noproxy", "*",
    "$url"
  )

  if (-not [string]::IsNullOrWhiteSpace($Ip)) {
    $args = @(
      "--silent",
      "--show-error",
      "--output", "NUL",
      "--write-out", $writeOut,
      "--max-time", "$Timeout",
      "--noproxy", "*",
      "--resolve", "$DomainName:443:$Ip",
      "$url"
    )
  }

  try {
    $raw = (& curl.exe @args).Trim()
    $parts = $raw -split "\|"
    if ($parts.Count -lt 3) {
      return (New-FailedRouteSample)
    }

    $code = $parts[0]
    $ttfb = [double]::Parse($parts[1], [System.Globalization.CultureInfo]::InvariantCulture)
    $total = [double]::Parse($parts[2], [System.Globalization.CultureInfo]::InvariantCulture)
    $ok = ($code -match "^\d{3}$" -and $code -ne "000")

    return [PSCustomObject]@{
      ok = $ok
      code = $code
      ttfb = $ttfb
      total = $total
    }
  } catch {
    return (New-FailedRouteSample)
  }
}

function Get-RouteScore {
  param([object]$Sample)
  return (0.7 * [double]$Sample.ttfb) + (0.3 * [double]$Sample.total)
}

function Get-SecondsSince {
  param([string]$UtcIso)
  if ([string]::IsNullOrWhiteSpace($UtcIso)) {
    return [double]::PositiveInfinity
  }
  try {
    $last = [DateTime]::Parse($UtcIso, [System.Globalization.CultureInfo]::InvariantCulture, [System.Globalization.DateTimeStyles]::RoundtripKind)
    return ((Get-Date).ToUniversalTime() - $last.ToUniversalTime()).TotalSeconds
  } catch {
    return [double]::PositiveInfinity
  }
}

function Get-PreferredIpCandidates {
  param(
    [string]$SingleIp,
    [string]$IpListRaw
  )

  $rawTokens = New-Object System.Collections.Generic.List[string]

  if (-not [string]::IsNullOrWhiteSpace($IpListRaw)) {
    foreach ($token in ($IpListRaw -split "[,;\s]+")) {
      if (-not [string]::IsNullOrWhiteSpace($token)) {
        $rawTokens.Add($token.Trim())
      }
    }
  } elseif (-not [string]::IsNullOrWhiteSpace($SingleIp)) {
    $rawTokens.Add($SingleIp.Trim())
  }

  $seen = @{}
  $candidates = New-Object System.Collections.Generic.List[string]
  foreach ($token in $rawTokens) {
    $parsed = $null
    if ([System.Net.IPAddress]::TryParse($token, [ref]$parsed)) {
      $normalized = $parsed.ToString()
      if (-not $seen.ContainsKey($normalized)) {
        $seen[$normalized] = $true
        $candidates.Add($normalized)
      }
    } else {
      Write-Log "ignore invalid ip token=$token"
    }
  }

  return ,$candidates.ToArray()
}

function Get-BestPreferredCandidate {
  param(
    [string]$DomainName,
    [string]$RequestPath,
    [int]$Timeout,
    [string[]]$Candidates
  )

  $bestIp = ""
  $bestSample = $null
  $bestScore = [double]::PositiveInfinity
  $summaryParts = New-Object System.Collections.Generic.List[string]

  foreach ($candidateIp in $Candidates) {
    $sample = Get-RouteSample -DomainName $DomainName -RequestPath $RequestPath -Timeout $Timeout -Ip $candidateIp
    $score = Get-RouteScore -Sample $sample
    $summaryParts.Add(("{0}:{1}/{2:N3}s" -f $candidateIp, $sample.code, $score))
    if ($sample.ok -and $score -lt $bestScore) {
      $bestIp = $candidateIp
      $bestSample = $sample
      $bestScore = $score
    }
  }

  if ($null -eq $bestSample) {
    $bestSample = New-FailedRouteSample
    $bestScore = Get-RouteScore -Sample $bestSample
  }

  return [PSCustomObject]@{
    hasOk = ($bestIp -ne "")
    ip = $bestIp
    sample = $bestSample
    score = $bestScore
    summary = ($summaryParts -join ", ")
  }
}

if (-not (Test-IsAdmin)) {
  throw "需要管理员权限运行（修改 hosts 必须管理员）。"
}

if ($IntervalMinutes -lt 1) { throw "IntervalMinutes 必须 >= 1" }
if ($TimeoutSec -lt 2) { throw "TimeoutSec 建议 >= 2" }
if ($FailureThreshold -lt 1) { throw "FailureThreshold 必须 >= 1" }
if ($BetterStreakToPin -lt 1) { throw "BetterStreakToPin 必须 >= 1" }
if ($WorseStreakToUnpin -lt 1) { throw "WorseStreakToUnpin 必须 >= 1" }
if ($CooldownMinutes -lt 0) { throw "CooldownMinutes 必须 >= 0" }
if ($BetterRatioThreshold -le 0) { throw "BetterRatioThreshold 必须 > 0" }
if ($BetterAbsoluteMs -lt 0) { throw "BetterAbsoluteMs 必须 >= 0" }

$state = Get-State
$nowUtc = (Get-Date).ToUniversalTime().ToString("o")
$state.lastCheckUtc = $nowUtc
$betterAbsoluteSec = $BetterAbsoluteMs / 1000.0
$cooldownSec = [double]($CooldownMinutes * 60)
$preferredCandidates = Get-PreferredIpCandidates -SingleIp $PreferredIp -IpListRaw $PreferredIpList

if ($preferredCandidates.Count -eq 0) {
  if ($state.mode -ne "dns") {
    Remove-ManagedHostsBlock
    Flush-DnsCache
    $state.mode = "dns"
    $state.lastSwitchUtc = $nowUtc
  }
  $state.currentPinnedIp = ""
  $state.betterStreak = 0
  $state.worseStreak = 0
  $state.pinnedFailStreak = 0
  Save-State -State $state
  Write-Log "no valid preferred ip candidates, keep dns mode"
  return
}

$isPinnedButMissingIp = ($state.mode -eq "pinned" -and [string]::IsNullOrWhiteSpace($state.currentPinnedIp))
if ($isPinnedButMissingIp) {
  Remove-ManagedHostsBlock
  Flush-DnsCache
  $state.mode = "dns"
  $state.currentPinnedIp = ""
  $state.betterStreak = 0
  $state.worseStreak = 0
  $state.pinnedFailStreak = 0
  $state.lastSwitchUtc = $nowUtc
  Write-Log "switch dns reason=missing_pinned_ip_in_state"
}

$pinnedIpNotAllowed = ($state.mode -eq "pinned" -and -not [string]::IsNullOrWhiteSpace($state.currentPinnedIp) -and -not ($preferredCandidates -contains $state.currentPinnedIp))
if ($pinnedIpNotAllowed) {
  $disallowedPinnedIp = $state.currentPinnedIp
  Remove-ManagedHostsBlock
  Flush-DnsCache
  $state.mode = "dns"
  $state.currentPinnedIp = ""
  $state.betterStreak = 0
  $state.worseStreak = 0
  $state.pinnedFailStreak = 0
  $state.lastSwitchUtc = $nowUtc
  Write-Log ("switch dns reason=pinned_ip_not_in_allow_list pinnedIp={0}" -f $disallowedPinnedIp)
}

$secondsSinceSwitch = Get-SecondsSince -UtcIso $state.lastSwitchUtc
$inCooldown = ($secondsSinceSwitch -lt $cooldownSec)

$dns = Get-RouteSample -DomainName $Domain -RequestPath $Path -Timeout $TimeoutSec
$bestPreferred = Get-BestPreferredCandidate -DomainName $Domain -RequestPath $Path -Timeout $TimeoutSec -Candidates $preferredCandidates

$preferredIpForCompare = ""
$pref = New-FailedRouteSample
$prefScore = Get-RouteScore -Sample $pref

if ($state.mode -eq "pinned" -and -not [string]::IsNullOrWhiteSpace($state.currentPinnedIp)) {
  $preferredIpForCompare = $state.currentPinnedIp
  if ($bestPreferred.hasOk -and $bestPreferred.ip -eq $preferredIpForCompare) {
    $pref = $bestPreferred.sample
    $prefScore = $bestPreferred.score
  } else {
    $pref = Get-RouteSample -DomainName $Domain -RequestPath $Path -Timeout $TimeoutSec -Ip $preferredIpForCompare
    $prefScore = Get-RouteScore -Sample $pref
  }
} elseif ($bestPreferred.hasOk) {
  $preferredIpForCompare = $bestPreferred.ip
  $pref = $bestPreferred.sample
  $prefScore = $bestPreferred.score
}

$dnsScore = Get-RouteScore -Sample $dns
$threshold = [Math]::Max(($dnsScore * $BetterRatioThreshold), $betterAbsoluteSec)
$preferredClearlyBetter = $false

if ($pref.ok -and -not $dns.ok) {
  $preferredClearlyBetter = $true
} elseif ($pref.ok -and $dns.ok -and -not [string]::IsNullOrWhiteSpace($preferredIpForCompare)) {
  $preferredClearlyBetter = (($dnsScore - $prefScore) -ge $threshold)
}

if ($preferredClearlyBetter) {
  $state.betterStreak = [int]$state.betterStreak + 1
  $state.worseStreak = 0
} elseif ($state.mode -eq "dns") {
  $state.betterStreak = 0
}

if ($state.mode -eq "dns") {
  if ($preferredClearlyBetter -and $state.betterStreak -ge $BetterStreakToPin -and -not $inCooldown -and -not [string]::IsNullOrWhiteSpace($preferredIpForCompare)) {
    Set-ManagedHostsBlock -EntryLine "$preferredIpForCompare`t$Domain"
    Flush-DnsCache
    $state.mode = "pinned"
    $state.currentPinnedIp = $preferredIpForCompare
    $state.worseStreak = 0
    $state.pinnedFailStreak = 0
    $state.lastSwitchUtc = $nowUtc
    Write-Log ("switch pinned domain={0} ip={1} dnsScore={2:N3}s prefScore={3:N3}s threshold={4:N3}s streak={5} candidates={6}" -f $Domain, $preferredIpForCompare, $dnsScore, $prefScore, $threshold, $state.betterStreak, $bestPreferred.summary)
  } else {
    Write-Log ("keep dns domain={0} dnsCode={1} bestPrefIp={2} prefCode={3} dnsScore={4:N3}s prefScore={5:N3}s threshold={6:N3}s betterStreak={7} inCooldown={8} candidates={9}" -f $Domain, $dns.code, $preferredIpForCompare, $pref.code, $dnsScore, $prefScore, $threshold, $state.betterStreak, $inCooldown, $bestPreferred.summary)
  }
} else {
  if (-not $pref.ok -and $dns.ok) {
    $state.pinnedFailStreak = [int]$state.pinnedFailStreak + 1
  } else {
    $state.pinnedFailStreak = 0
  }

  $prefStillClearlyBetter = $preferredClearlyBetter
  if ($prefStillClearlyBetter) {
    $state.worseStreak = 0
  } else {
    $state.worseStreak = [int]$state.worseStreak + 1
  }

  $mustFallback = ($state.pinnedFailStreak -ge $FailureThreshold)
  $shouldFallback = (-not $inCooldown -and $state.worseStreak -ge $WorseStreakToUnpin -and $dns.ok)

  if ($mustFallback -or $shouldFallback) {
    Remove-ManagedHostsBlock
    Flush-DnsCache
    $state.mode = "dns"
    $state.currentPinnedIp = ""
    $state.betterStreak = 0
    $state.worseStreak = 0
    $state.pinnedFailStreak = 0
    $state.lastSwitchUtc = $nowUtc
    Write-Log ("switch dns domain={0} reason={1} pinnedIp={2} dnsCode={3} prefCode={4} dnsScore={5:N3}s prefScore={6:N3}s candidates={7}" -f $Domain, ($(if ($mustFallback) { "pin_fail" } else { "not_clearly_better" })), $preferredIpForCompare, $dns.code, $pref.code, $dnsScore, $prefScore, $bestPreferred.summary)
  } else {
    Write-Log ("keep pinned domain={0} pinnedIp={1} dnsCode={2} prefCode={3} dnsScore={4:N3}s prefScore={5:N3}s worseStreak={6} pinFailStreak={7} inCooldown={8} candidates={9}" -f $Domain, $preferredIpForCompare, $dns.code, $pref.code, $dnsScore, $prefScore, $state.worseStreak, $state.pinnedFailStreak, $inCooldown, $bestPreferred.summary)
  }
}

Save-State -State $state
