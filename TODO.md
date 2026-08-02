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

## P2 — Media prioridad

- [ ] **Output directory picker**
  - En el edit popup, poder navegar a un directorio de salida
  - Actualmente solo permite escribir la ruta a mano

- [ ] **Recursive directory processing**
  - Flag para procesar archivos en subdirectorios
  - Útil para lotes grandes

- [ ] **Progress bar real**
  - Parsear salida `-monitor` de ImageMagick
  - Mostrar barra de progreso en lugar de solo spinner

## P3 — Nice to have

- [ ] **Image preview in terminal**
  - Mostrar miniatura vía protocolo Kitty / sixel
  - Killer feature para herramienta de imágenes

- [x] **Before/after comparison**
  - Tecla `b` para ver comparativa lado a lado original vs procesado
  - Procesa copia temporal, respeta formato, directorio y args extra

- [ ] **EXIF metadata panel**
  - Mostrar metadatos EXIF adicionales (cámara, ISO, fecha, GPS)

- [ ] **Undo / revert**
  - Borrar archivos generados con una tecla

- [ ] **Tema / colores personalizables**
  - Permitir cambiar colores de la TUI vía settings.toml

---

*Creado: 2026-08-01 | Actualizado: 2026-08-02*