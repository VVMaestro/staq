# STAQ

## License

STAQ is proprietary freeware. Official, unmodified releases may be used free
of charge for personal and internal business purposes. Redistribution, resale,
modification, and source code reuse require separate written permission.
See [LICENSE](LICENSE) for the full terms. Third-party components retain their
own licenses.

Приложение STAQ бесплатно для личного использования и внутреннего использования
в организациях. Распространение, перепродажа, изменение программы и использование
исходного кода в других проектах требуют отдельного письменного разрешения.
Полные условия приведены в [LICENSE](LICENSE). Сторонние компоненты сохраняют
свои лицензии.

## Development

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Windows window

`src-tauri/tauri.windows.conf.json` enables a transparent, undecorated window
with a minimum size of 360×480. Vue draws the opaque surface with 10 px corners
and a 32 px title bar. Maximized windows have square corners. There is no outer
shadow or transparent padding; transparent corners do not promise click-through.

Drag the title bar to move the window and double-click it to maximize or restore.
The window buttons and resize edges remain available while the task dialog is open.
On Windows, closing the window (including Alt+F4) hides it to the system tray.
The app keeps running, preserving the open dialog and any text entered in it.
Left-click the STAQ tray icon to restore the window; right-click for
«Открыть STAQ» and «Выход». «Выход» fully exits the app. The minimize button
still minimizes to the taskbar. Launching STAQ again restores the existing window.
If tray creation fails, closing the window exits normally. Browser previews
hide the window controls. Native Snap Layouts on maximize-button hover are not implemented.

Run `bun run tauri dev` to check native behavior, or
`bun run tauri build --debug --no-bundle` to build a debug executable. After changing
the frame, check moving/resizing, maximize/restore, Alt+F4, repeated launch after
minimizing, transparent corners with and without the dialog, and Windows DPI scales
100%, 125%, and 150%. In the dialog, check Tab/Shift+Tab, Escape, Enter submission,
and return of focus to the button that opened it.

For the tray, check repeated close/restore cycles, Alt+F4, restoring a maximized
window, and closing with an unfinished dialog. Verify there is only one process
and tray icon after a repeated launch. Test «Выход» with the window both visible
and hidden, then relaunch and verify saved tasks remain available.

## Editor extensions

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
