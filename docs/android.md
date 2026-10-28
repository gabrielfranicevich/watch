# Guía de instalación y configuración de Watch en Android

Estos pasos permiten instalar `watch` desde cero en Android usando Termux, configurar MPV-Android y reproducir anime con los parámetros necesarios para los streams.

## 1. Instalar las aplicaciones

Instalá estas dos aplicaciones:

- **Termux:** terminal donde se instala y ejecuta `watch`. Preferentemente, descargalo desde [F-Droid](https://f-droid.org/packages/com.termux/).
- **MPV-Android:** reproductor de video. Está disponible en [GitHub — MPV-Android](https://github.com/mpv-android/mpv-android/releases).

Abrí Termux y actualizá los paquetes:

```sh
pkg update && pkg upgrade
```

Instalá las dependencias:

```sh
pkg install rust git
```

Verificá que Cargo esté disponible:

```sh
cargo --version
```

## 2. Instalar Watch

La primera vez, instalá el ejecutable desde crates.io con Cargo:

```sh
cargo install watch
```

Si el paquete todavía no está publicado o Cargo no encuentra la versión de este proyecto, instalalo directamente desde la rama Rust del repositorio:

```sh
cargo install --git https://github.com/gabrielfranicevich/watch --branch develop
```

Para futuras actualizaciones, una vez instalado:

```sh
watch -U
```

Verificá que el comando esté disponible:

```sh
watch --help
```

## 3. Configurar MPV-Android

MPV-Android debe cargar el archivo de configuración que `watch` genera para cada reproducción. Ese archivo contiene opciones del stream, como `Referer` y subtítulos externos.

Desde Termux, creá el directorio compartido:

```sh
mkdir -p /storage/emulated/0/mpv
```

El archivo que `watch` actualiza para los parámetros de cada reproducción es:

```text
/storage/emulated/0/mpv/mpv.config.mp4
```

`watch` lo genera o actualiza cuando corresponde. No hace falta completar manualmente las URLs de cada anime.

Para que MPV-Android lo cargue:

1. Abrí MPV-Android.
2. Entrá en **Settings → Advanced → Edit mpv.conf** (el nombre puede variar según la versión).
3. Agregá esta línea, conservando las demás opciones existentes:

   ```text
   include=/storage/emulated/0/mpv/mpv.config.mp4
   ```

4. Guardá los cambios y reiniciá MPV-Android si la aplicación lo requiere.

## 4. Dar los permisos necesarios

Revisá los permisos de MPV-Android en **Ajustes de Android → Aplicaciones → MPV-Android → Permisos**. Concedé los permisos de archivos multimedia que estén disponibles.

Los nombres y opciones varían entre versiones de Android y del reproductor. No todos los dispositivos muestran una opción llamada “Allow access to manage all files”.

Si Termux no puede crear o actualizar los archivos del directorio compartido, ejecutá `termux-setup-storage`, aceptá el permiso de Android y volvé a probar.

## 5. Probar la reproducción

Desde Termux, buscá un anime:

```sh
watch beerus
```

Seleccioná el resultado y el episodio. Si Android muestra un selector de aplicaciones, elegí MPV.

Para comprobar MPV y la apertura mediante `termux-open` de forma independiente, probá este stream público de demostración:

```sh
termux-open --chooser \
  --content-type 'video/*' \
  'https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8'
```

Si MPV reproduce ese video, la apertura mediante `termux-open` funciona. La disponibilidad de streams de anime depende además del proveedor.

## 6. Actualizar Watch

No hace falta repetir toda la instalación. Ejecutá:

```sh
watch -U
```

Después podés seguir usando:

```sh
watch nombre-del-anime
```

Mantené la línea `include` en el `mpv.conf` de MPV-Android. Si reinstalás el reproductor, restablecés su configuración o borrás sus datos, verificá que siga presente.
