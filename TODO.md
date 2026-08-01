# TODO — lazymagick

> Port desde lazyffmpeg completado ✅ (ver PLAN.md)
> Próximas features priorizadas

---

## P1 — Alta prioridad

- [ ] **CLI / batch headless mode**
  - Poder ejecutar `lazymagick -r "weight medium" -f avif *.png` sin abrir la TUI
  - Útil para scripts y automatización
  - Añadir clap/structopt como dep opcional

- [ ] **Export built-in recipes a `~/.config/`**
  - Tecla (ej: `E`) que copia todas las built-in a `~/.config/lazymagick/recipes/`
  - Así el usuario puede editarlas cómodamente

- [ ] **Búsqueda / filtro de recetas**
  - Con 42 recetas, teclear para filtrar por nombre/categoría
  - Input inline en el recipe panel (como fzf)

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

- [ ] **Before/after comparison**
  - Lado a lado original vs resultado procesado

- [ ] **EXIF metadata panel**
  - Mostrar metadatos EXIF adicionales (cámara, ISO, fecha, GPS)

- [ ] **Undo / revert**
  - Borrar archivos generados con una tecla

- [ ] **Tema / colores personalizables**
  - Permitir cambiar colores de la TUI vía settings.toml

---

*Creado: 2026-08-01*