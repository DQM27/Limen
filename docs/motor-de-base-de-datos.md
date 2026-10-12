# Motor de base de datos: SurrealKV frente a RocksDB

Informe del experimento de la rama `exp/rocksdb`. **No cambia el motor por defecto**: el
dueño decide. Separa lo medido de lo supuesto.

## Resumen

- **Recomendación: seguir con SurrealKV.** Ningún motor perdió un solo hecho en ninguna
  prueba, así que la durabilidad no inclina la balanza hoy; RocksDB cuesta más
  (compilación, tamaño) y cierra el camino del cifrado en reposo.
- **Lo que no queda descartado:** el reporte #428 de `surrealkv` (un commit confirmado
  puede perderse tras un cierre brusco si coincide con la rotación de la memoria de
  escritura). La prueba más fuerte que se pudo armar no lo reprodujo, pero **no lo descarta**
  (ver "Qué no demuestra").
- **Qué cambiaría la recomendación:** que SurrealKV pierda algo en una prueba de rotación
  y RocksDB no; o que se confirme el problema en la versión que se vaya a publicar.

## Entorno

| | Esta PC (desarrollo) | GitHub Actions |
|---|---|---|
| Rust | 1.99.0, toolchain **GNU** (no hay Visual Studio) | 1.99.0, Linux y Windows **MSVC** |
| Compilador de C++ | `g++` 15.2 y `clang` 19.1.1 (`mingw64`), `cmake` 3.30.4 | Linux: `g++` 13.3; Windows: MSVC |
| Máquina | i3-1215U, 8 hilos, 15,7 GB | 4 hilos (máquina virtual) |
| Motor y versión | `surrealdb` 3.3.2; SurrealKV 0.21.4 | igual |

## 1. Compilar RocksDB

### En esta PC: compila, pero el programa se cae al abrir la base

Pasos y errores, en el orden en que aparecieron (los errores completos están en la
conversación de la sesión; aquí el resumen):

1. **Desde Git Bash:** `bindgen` no podía cargar `libclang.dll`
   (`LoadLibraryExW failed`). Ninguna DLL faltaba; el `PATH` de Git Bash pone su propio
   `/mingw64/bin` antes que el de `mingw64` y carga una librería equivocada. **Hay que
   compilar desde PowerShell con `C:\Users\femprobrisas\AppData\Local\mingw64\bin` al
   principio del `PATH`.**
2. **En debug:** `g++` falla al ensamblar `rocksdb/db/db_impl/db_impl.cc`
   (`Fatal error: can't write 29 bytes to section ... 'file too big'`): el C++ se compila con
   `-O0 -g` y ese archivo supera el límite del formato de objeto. **Se resuelve con
   `CXXFLAGS="-O2 -g0"`** (sólo `CXXFLAGS`: con `CFLAGS=-O2` se rompe `aws-lc-sys`, que
   prohíbe optimizar su módulo `jitterentropy`). En **release** compila sin trucos.
3. **Al ejecutar:** el programa se cae al abrir la base con `0xC0000005` (violación de
   acceso), en debug y en release. Traza (`gdb`): SIGSEGV en la dirección 0 dentro de
   `pthread_once`, llamado desde `rocksdb::Env::Default()` ← `rocksdb_options_create`.
   SurrealKV dentro del mismo ejecutable funciona.
   - *Supuesto, no verificado:* el C++ lo compila el `g++` de `mingw64` y lo enlaza el
     MinGW propio de Rust, y los dos no se entienden en los hilos (`pthread_once`).
     Se podría probar `-C link-self-contained=no`; no se probó.

En **GitHub con MSVC** (el toolchain que usaría un instalador real) RocksDB compila y abre
sin problema. Es lo que importa para producción; el fallo de esta PC es de su toolchain.

### Tiempos y tamaños

| Medida | SurrealKV | RocksDB |
|---|---|---|
| Compilación en frío, debug (esta PC) | 15 min 24 s | no medida limpia: ~38 min sumando intentos (cota superior) |
| Compilación en frío, **release** (esta PC) | 21 min 04 s | **36 min 34 s** |
| Compilación en frío, release (GitHub Linux / Windows) | – | 19,5 min / 38,5 min |
| Incremental en debug (tocar un archivo de `infra-surreal`) | 24 a 32 s | 39 a 45 s |
| Carpeta `target` tras debug | 3589 MB | 3947 MB |
| Carpeta `target` tras debug + release | 5372 MB | 5991 MB |
| Ejecutable de prueba en release | 150,1 MB | 172,6 MB (incluye los dos motores) |

Notas: los tiempos de esta PC están algo inflados por `rust-analyzer` compilando a la vez;
el ejecutable de RocksDB aquí lleva también SurrealKV, así que la diferencia de ~22 MB es el
costo de añadir RocksDB, no de sustituir. El incremental sube porque se enlaza un ejecutable
más grande: el C++ no se recompila.

## 2. Pruebas de `infra-surreal` con base en disco

Corridas de GitHub (Linux y Windows MSVC), `cargo test -p limen-infra-surreal --features
rocksdb`: **pasan** las 9 pruebas de integración, incluida
`los_datos_persisten_en_disco_con_rocksdb` (nueva), junto con la equivalente de SurrealKV.
No se corrieron en esta PC con RocksDB porque aquí se cae al abrir.

## 3. Cierre brusco

Programa: `experimentos/cierre-brusco`. Un proceso hijo abre la base y confirma hechos
(`UnidadDeTrabajo`, como en la app) imprimiendo cuántos confirmó; el padre lo mata
(`taskkill /F` en Windows, `kill -9` en Linux), y otro proceso reabre la base y cuenta
cuántos hechos hay. **Perdidas = confirmadas − encontradas.**

- **Tamaño de la escritura:** ~600 B por fila en disco (medido: 79 000 filas = 48,5 MB),
  más que los ~300 B que se supusieron.
- **Escenarios:** `pequeña` (1 fila por transacción), `grande` (500 filas) y `rotación`
  (5000 filas por transacción; el cierre se dispara a 0–20 ms de que el motor crea un
  archivo nuevo en la carpeta de datos, es decir, en la rotación).
- **Escenarios descartados:** los de "memoria de escritura de 1 MiB y 8 MiB". La
  configuración **no llega al motor embebido** (ni la variable `SURREAL_…` ni los
  parámetros en la ruta): en 10 s se escribieron ~48 MB y no apareció ningún archivo de
  volcado con ni sin ella. Sus resultados no valen y no se cuentan.
- Con el tamaño por defecto (con 15,7 GB de RAM: 256 MB en SurrealKV, 64 MiB en RocksDB)
  los escenarios `pequeña` y `grande` **nunca rotan** (una repetición escribe entre 1 y
  12 MB).

### Resultados (100 repeticiones por escenario, salvo rotación: 50)

| Dónde | Motor | pequeña | grande | rotación |
|---|---|---|---|---|
| Esta PC (Windows GNU, `taskkill /F`) | SurrealKV | 0 pérdidas | 0 pérdidas | 0 pérdidas |
| GitHub Linux (`kill -9`) | SurrealKV | 0 | 0 | 0 |
| GitHub Linux (`kill -9`) | RocksDB | 0 | 0 | 0 |
| GitHub Windows MSVC (`taskkill /F`) | SurrealKV | 0 | 0 | 0 |
| GitHub Windows MSVC (`taskkill /F`) | RocksDB | 0 | 0 | 0 |

En la rotación, el archivo que disparó el cierre fue: SurrealKV un `.sst` en 20 de 50
(Linux) y 19 de 50 (Windows) y un archivo del registro de escrituras en el resto; RocksDB un
archivo del registro de escrituras en las 50, en los dos sistemas. En todas las
repeticiones lo disparó un archivo, no el tiempo. Cada repetición de rotación confirmó
unas 320 000 filas con SurrealKV y 90 000 con RocksDB antes de rotar.

### Qué no demuestra

- **Matar el proceso no es cortar la energía.** Al terminar el proceso, el sistema
  operativo conserva lo que ya recibió aunque no esté en el disco. Esta prueba mide la
  lógica del motor ante un cierre brusco; **no** mide un corte eléctrico, que es lo que de
  verdad puede dañar un disco o perder lo que no se sincronizó. El reporte #428 tampoco
  probó cortes de energía.
- **"0 de 50" no es "nunca":** con 0 fallos en 50 repeticiones, la frecuencia del problema,
  si existe, podría ser de hasta ~6 % de las repeticiones (95 % de confianza).
- **El instante puede no coincidir** con el commit exacto del reporte: se mata a 0–20 ms
  de que aparece el archivo nuevo, no se garantiza caer sobre esa confirmación.
- **Máquinas virtuales de GitHub:** la velocidad del disco y de `fsync` no es la de la
  portería.
- Ambos motores se probaron con su durabilidad por defecto (sincronizar al confirmar cada
  transacción).

## 4. Consideraciones que no salen de las pruebas

- **Cifrado en reposo** (`arquitectura.md`, 8.2): SurrealKV tiene cifrado (TDE) en su rama
  principal, aún sin publicar ni conectar en SurrealDB; el de RocksDB no está conectado en
  SurrealDB. Pasar a RocksDB cierra ese camino.
- **Android:** RocksDB complica la compilación cruzada (no se probó aquí).
- **Madurez:** la documentación oficial marca SurrealKV como **beta** y recomienda RocksDB
  para un servidor de producción prudente
  ([arquitectura](https://surrealdb.com/docs/surrealdb/introduction/architecture)).
  La corrección del #428 está sólo en `main` de `surrealkv`; la última publicada sigue
  siendo la 0.21.4. Existe `surrealdb` 3.3.3 (se usa la 3.3.2): revisar sus notas antes de
  subir.

## Recomendación

1. **Mantener SurrealKV por defecto.** Sin pérdidas en ninguna prueba y con menos costo de
   compilación, tamaño y complejidad; conserva el camino del cifrado.
2. **Cerrar ordenado cuando se pueda** y **actualizar** `surrealkv`/`surrealdb` apenas
   salga la versión con las correcciones (junto con el cifrado).
3. **Si el riesgo no se acepta:** RocksDB es viable **compilando con MSVC** (en la CI sí,
   en esta PC no sin Visual Studio o sin resolver el enlace GNU). Costaría más tiempo de
   compilación, ~22 MB más y el cifrado en reposo.
4. **Pendiente si se quiere más certeza:** una prueba con **corte de energía real** (máquina
   virtual con el disco cortado o equipo físico) y más repeticiones de rotación.

## Cómo repetirlo

- Código: `experimentos/cierre-brusco` y el workflow `.github/workflows/exp-rocksdb.yml`
  (sólo en la rama `exp/rocksdb`).
- En esta PC, desde PowerShell: `RUSTUP_TOOLCHAIN=1.99.0-x86_64-pc-windows-gnu`,
  `mingw64\bin` primero en el `PATH` y, para debug con RocksDB, `CXXFLAGS="-O2 -g0"`.
- `limen-exp-cierre-brusco padre <motor> <filas> <repeticiones> <max_ms> [rotacion]`.
