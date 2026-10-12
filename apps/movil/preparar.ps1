# Arma el APK de prueba de Limen móvil (arm64) y lo deja en el Escritorio.
#
#   .\preparar.ps1 [-Base <carpeta "base" a sembrar>] [-SinRust]
#
# 1. Compila la interfaz móvil (`ui/`, Angular) y la copia a `web/`.
# 2. Comprime la base de pruebas como `web/semilla.zip` (sin el archivo LOCK ni
#    la identidad del equipo: el teléfono crea la suya). `web/` no se sube al
#    repositorio: lleva datos reales del cliente.
# 3. Compila el núcleo de Rust con `cargo ndk` y copia el `.so` a `jniLibs`,
#    como hace Lattis. No usa `tauri android build`: ese crea enlaces
#    simbólicos, que Windows sólo permite con el Modo de desarrollador.
# 4. Arma el APK con Gradle (JDK 21: la plantilla de Tauri usa Gradle 8, que no
#    corre con JDK 26) y lo copia al Escritorio.
#
# `-SinRust` salta el paso 3 si la librería ya está compilada y sólo cambió la
# interfaz o la semilla.

param(
    [string]$Base = "$env:APPDATA\com.dqm27.limen.escritorio\base",
    [switch]$SinRust
)

$ErrorActionPreference = 'Stop'
$raiz = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$ui = Join-Path $PSScriptRoot 'ui'
$web = Join-Path $PSScriptRoot 'web'
$android = Join-Path $PSScriptRoot 'src-tauri\gen\android'
$jdk = "$env:LOCALAPPDATA\jdk-21"

# Esta PC compila con el toolchain GNU (ver docs/continuar.md).
$env:RUSTUP_TOOLCHAIN = '1.99.0-x86_64-pc-windows-gnu'
$env:ANDROID_NDK_HOME = Get-ChildItem "$env:ANDROID_HOME\ndk" | Sort-Object Name | Select-Object -Last 1 -ExpandProperty FullName
$env:JAVA_HOME = $jdk
$env:PATH = "$jdk\bin;$env:PATH"

Push-Location $ui
if (-not (Test-Path node_modules)) { npm install }
npm run build
Pop-Location

if (Test-Path $web) { Remove-Item $web -Recurse -Force }
Copy-Item "$ui\dist\limen-movil\browser" $web -Recurse

$temporal = Join-Path $env:TEMP 'limen-semilla'
if (Test-Path $temporal) { Remove-Item $temporal -Recurse -Force }
# `-Exclude` no sirve con carpetas: se copia todo y se quita LOCK (la app
# del escritorio lo tiene tomado mientras está abierta; cerrarla antes).
Copy-Item $Base $temporal -Recurse
Remove-Item "$temporal\LOCK" -ErrorAction SilentlyContinue
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::CreateFromDirectory($temporal, "$web\semilla.zip", 'Optimal', $false)
Remove-Item $temporal -Recurse -Force

$jni = Join-Path $android 'app\src\main\jniLibs\arm64-v8a'
if (-not $SinRust) {
    Push-Location $raiz
    cargo ndk -t arm64-v8a build --release -p limen-movil --features custom-protocol
    Pop-Location
    New-Item -ItemType Directory -Force $jni | Out-Null
    Copy-Item "$raiz\target\aarch64-linux-android\release\liblimen_movil_lib.so" $jni -Force
}

Push-Location $android
.\gradlew.bat assembleArm64Debug -x rustBuildArm64Debug
Pop-Location

$apk = Get-ChildItem "$android\app\build\outputs\apk" -Recurse -Filter *.apk | Select-Object -First 1
$destino = Join-Path ([Environment]::GetFolderPath('Desktop')) 'limen-movil.apk'
Copy-Item $apk.FullName $destino -Force
"APK listo: $destino ($([math]::Round((Get-Item $destino).Length / 1MB, 1)) MB)"
