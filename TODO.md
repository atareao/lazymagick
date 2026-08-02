# TODO — lazymagick

> Port desde lazyffmpeg completado ✅ (ver PLAN.md)
> Phase 2 — P1 Features completado ✅ (CLI, Export, Search)

---

## P1 — Alta prioridad ✅

- [x] **CLI / batch headless mode**
  - `lazymagick -r "weight medium" -f avif *.png` sin abrir la TUI
  - Vía clap + glob, con `-r/--recipe`, `-f/--format`, `-o/--output`, `--dry-run`

- [x] **Export built-in recipes a `~/.config/`**
  - Tecla `E` que copia `builtins.toml` a `~/.config/lazymagick/recipes/`
  - Recarga las recetas de usuario automáticamente

- [x] **Búsqueda / filtro de recetas**
  - Escribe cualquier carácter en el panel de recetas para filtrar
  - Filtra por nombre, categoría y tags (case-insensitive)
  - `Esc` limpia, `Enter` selecciona el primer match

## P2 — Media prioridad ✅

- [x] **Output directory picker**
  - Ctrl+O para navegar directorio de salida desde edit popup

- [x] **Recursive directory processing**
  - Tecla `R` para procesar archivos en subdirectorios

- [x] **Progress bar real**
  - Parseo de `-monitor` de ImageMagick con barra y porcentaje

## P3 — Nice to have ✅

- [x] **Image preview in terminal**
  - Tecla `p` para previsualizar imagen vía Kitty/Sixel/Halfblocks
  - Usa `ratatui-image` con detección automática de protocolo

- [x] **Before/after comparison**
  - Tecla `b` para ver comparativa lado a lado original vs procesado
  - Procesa copia temporal, respeta formato, directorio y args extra

- [x] **EXIF metadata panel**
  - Tecla `x` para ver metadatos de cámara, ISO, GPS, etc.

- [x] **Undo / revert**
  - Tecla `u` para ver y borrar archivos generados

- [x] **Tema / colores personalizables**
  - 16 tokens de color configurables vía `settings.toml`

---

*Creado: 2026-08-01 | Actualizado: 2026-08-02*