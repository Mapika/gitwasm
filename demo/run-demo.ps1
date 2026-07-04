# gitwasm end-to-end demo.
# 1. Builds the host CLI and the wasm modules.
# 2. Creates a playground repo whose .gitwasm/ carries the modules.
# 3. Shows a package-lock.json merge that plain git ALWAYS conflicts on
#    merging cleanly through the sandboxed wasm merge driver.
# 4. Shows the sandboxed pre-commit secret scanner blocking a leaked AWS key.

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent

function Step($msg) { Write-Host "`n=== $msg ===" -ForegroundColor Cyan }

Step "Building host CLI + wasm modules"
cargo build --release -p gitwasm --manifest-path "$root\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw "host build failed" }
cargo build --release -p lockfile-merge -p secret-scan --target wasm32-wasip1 --manifest-path "$root\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw "module build failed" }

$env:PATH = "$root\target\release;$env:PATH"

Step "Creating playground repo with committed .gitwasm/"
$play = Join-Path $PSScriptRoot "playground"
if (Test-Path $play) { Remove-Item -Recurse -Force $play }
New-Item -ItemType Directory -Force "$play\.gitwasm" | Out-Null
Copy-Item "$root\target\wasm32-wasip1\release\lockfile-merge.wasm" "$play\.gitwasm\"
Copy-Item "$root\target\wasm32-wasip1\release\secret-scan.wasm" "$play\.gitwasm\"

Set-Content -Path "$play\.gitwasm\manifest.toml" -Value @'
[hooks]
pre-commit = "secret-scan.wasm"

[[merge]]
pattern = "package-lock.json"
module = "lockfile-merge.wasm"
'@

Set-Content -Path "$play\.gitattributes" -Value "package-lock.json merge=gitwasm"

Set-Content -Path "$play\package-lock.json" -Value @'
{
  "name": "demo-app",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "packages": {
    "": {
      "name": "demo-app",
      "dependencies": {
        "express": "^4.19.0"
      }
    },
    "node_modules/express": {
      "version": "4.19.2",
      "resolved": "https://registry.npmjs.org/express/-/express-4.19.2.tgz"
    }
  }
}
'@

Push-Location $play
try {
    git init -b main -q
    git config user.email "demo@gitwasm.dev"
    git config user.name "gitwasm demo"
    git add -A

    Step "Activating repo-embedded behavior (one command per clone)"
    gitwasm install
    if ($LASTEXITCODE -ne 0) { throw "gitwasm install failed" }

    git commit -q -m "initial commit (passes wasm pre-commit scan)"
    if ($LASTEXITCODE -ne 0) { throw "initial commit failed" }

    Step "Branch 'feature' adds dependency left-pad"
    git checkout -q -b feature
    (Get-Content package-lock.json -Raw) `
        -replace '"express": "\^4\.19\.0"', ('"express": "^4.19.0",' + "`n        " + '"left-pad": "^1.3.0"') `
        -replace '"node_modules/express": \{', ('"node_modules/left-pad": {' + "`n      " + '"version": "1.3.0",' + "`n      " + '"resolved": "https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz"' + "`n    },`n    " + '"node_modules/express": {') `
        | Set-Content package-lock.json
    git commit -q -am "add left-pad"

    Step "Branch 'main' adds dependency right-pad (same lines - guaranteed textual conflict)"
    git checkout -q main
    (Get-Content package-lock.json -Raw) `
        -replace '"express": "\^4\.19\.0"', ('"express": "^4.19.0",' + "`n        " + '"right-pad": "^1.0.1"') `
        -replace '"node_modules/express": \{', ('"node_modules/right-pad": {' + "`n      " + '"version": "1.0.1",' + "`n      " + '"resolved": "https://registry.npmjs.org/right-pad/-/right-pad-1.0.1.tgz"' + "`n    },`n    " + '"node_modules/express": {') `
        | Set-Content package-lock.json
    git commit -q -am "add right-pad"

    Step "Control: what plain git line-merge does with these three versions"
    git merge-file -p (git rev-parse --git-dir) 2>$null | Out-Null  # noop to keep PS happy
    $baseF = New-TemporaryFile; $oursF = New-TemporaryFile; $theirsF = New-TemporaryFile
    git show "HEAD~1:package-lock.json" | Set-Content $baseF
    git show "HEAD:package-lock.json"   | Set-Content $oursF
    git show "feature:package-lock.json" | Set-Content $theirsF
    git merge-file -p $oursF $baseF $theirsF > $null 2>&1
    Write-Host "git merge-file exit code: $LASTEXITCODE  (>0 = conflicts, this is what everyone suffers)" -ForegroundColor Yellow
    Remove-Item $baseF, $oursF, $theirsF

    Step "Now the real merge - through the sandboxed wasm merge driver"
    git merge feature -m "merge feature"
    if ($LASTEXITCODE -ne 0) { throw "merge conflicted - demo failed" }

    $merged = Get-Content package-lock.json -Raw
    if ($merged -match "left-pad" -and $merged -match "right-pad") {
        Write-Host "MERGED CLEAN: lockfile contains BOTH left-pad and right-pad, zero conflict markers" -ForegroundColor Green
    } else {
        throw "merged file is missing a dependency - demo failed"
    }

    Step "Secret scanner: attempt to commit a leaked AWS key"
    Set-Content config.js 'const awsKey = "AKIAIOSFODNN7EXAMPLE";'
    git add config.js
    git commit -q -m "add config" 2>&1 | Out-String | Write-Host
    if ($LASTEXITCODE -ne 0) {
        Write-Host "COMMIT BLOCKED by sandboxed wasm pre-commit hook - as intended" -ForegroundColor Green
    } else {
        throw "commit with secret went through - demo failed"
    }

    Step "Fix the leak and commit again"
    Set-Content config.js 'const awsKey = process.env.AWS_ACCESS_KEY_ID;'
    git add config.js
    git commit -q -m "add config (key from env)"
    if ($LASTEXITCODE -ne 0) { throw "clean commit was blocked - demo failed" }
    Write-Host "Clean commit accepted" -ForegroundColor Green

    Step "Demo complete"
    Write-Host "The merge driver and the hook are wasm blobs COMMITTED IN THE REPO,"
    Write-Host "running sandboxed (no filesystem, no network, no env beyond what the host hands them)."
    Write-Host "Anyone who clones this repo and runs 'gitwasm install' gets identical behavior on any OS."
}
finally {
    Pop-Location
}
