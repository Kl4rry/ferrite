use std::{
    borrow::Cow,
    collections::HashMap,
    fs::FileType,
    str::FromStr,
    sync::{Arc, LazyLock},
};

use ferrite_geom::rect::Rect;
use ferrite_runtime::{Bounds, View};
use ferrite_style::Color;
use ferrite_utility::tui_buf_ext::TuiBufExt;
use unicode_width::UnicodeWidthStr;

use crate::{
    config::editor::Editor,
    file_explorer::{DirEntry, FileExplorer},
    theme::EditorTheme,
};

pub struct FileExplorerView {
    theme: Arc<EditorTheme>,
    config: Arc<Editor>,
    has_focus: bool,
}

impl FileExplorerView {
    pub fn new(config: Arc<Editor>, theme: Arc<EditorTheme>, has_focus: bool) -> Self {
        Self {
            theme,
            config,
            has_focus,
        }
    }
}

impl View<FileExplorer> for FileExplorerView {
    fn render(
        &self,
        state: &mut FileExplorer,
        bounds: Bounds,
        painter: &mut ferrite_runtime::Painter,
    ) {
        let unique_id = state.unique_id();
        let layer = painter.create_layer(unique_id, bounds);
        let mut layer = layer.lock().unwrap();
        let area = bounds.grid_bounds();
        let buf = &mut layer.buf;
        if area.area() == 0 {
            return;
        }

        buf.set_style(area.into(), self.theme.background);

        let text_style = self.theme.text;
        let dir_style = self.theme.file_explorer_directory;
        let exe_style = self.theme.file_explorer_executable;
        let link_style = self.theme.file_explorer_link;

        if area.height > 2 {
            let height = area.height.saturating_sub(1);
            let page = state.index() / height;
            let start = page * height;

            let entries = state.entries();
            for i in 0..height {
                let index = start + i;
                let Some(entry) = entries.get(index) else {
                    continue;
                };
                let Some(file_name) = entry.path.file_name() else {
                    continue;
                };
                let mut file_name = file_name.to_string_lossy();
                let is_dir = entry.file_type.is_dir();
                if is_dir {
                    let mut file = file_name.into_owned();
                    file.push('/');
                    file_name = file.into();
                }

                let (icon, color) = get_icon(entry, entry.file_type);
                let icon_width = if self.config.icons {
                    icon.width() + 2
                } else {
                    0
                };

                let icon_style = match color {
                    Some(color) => {
                        let mut tmp_style = text_style;
                        tmp_style.fg = Some(color);
                        tmp_style
                    }
                    None if is_dir => dir_style,
                    None => text_style,
                };
                buf.draw_string(
                    (area.x + 1) as u16,
                    (area.y + i) as u16,
                    icon,
                    area.into(),
                    icon_style,
                );

                let style = if entry.link.is_some() {
                    link_style
                } else if is_dir {
                    dir_style
                } else if is_executable(&entry.metadata) {
                    exe_style
                } else {
                    text_style
                };

                let line: Cow<str> = match &entry.link {
                    Some(path) => format!("{} -> {}", file_name, path.to_string_lossy()).into(),
                    None => file_name,
                };

                buf.draw_string(
                    (area.x + icon_width) as u16,
                    (area.y + i) as u16,
                    &line,
                    area.into(),
                    style,
                );
                if i + start == state.index() {
                    buf.set_style(
                        Rect::new(area.x, area.y + i, area.width, 1).into(),
                        self.theme.selection,
                    );
                }
            }
        }

        if area.height > 1 {
            let view_bounds = bounds.view_bounds();
            let cell_size = bounds.cell_size();
            let bottom_line_bounds = Bounds::new(
                Rect::new(
                    view_bounds.x,
                    (view_bounds.y as f32 + view_bounds.height as f32 - cell_size.y).round()
                        as usize,
                    view_bounds.width,
                    (1.0 * cell_size.y).round() as usize,
                ),
                cell_size,
                bounds.rounding,
            );

            let layer = painter.create_layer(("file-explorer-bottom-line", 10), bottom_line_bounds);
            let mut layer = layer.lock().unwrap();
            let buf = &mut layer.buf;
            let info_line_area = bottom_line_bounds.grid_bounds();

            let info_line_y = info_line_area.y + info_line_area.height - 1;

            // Its a bit bruh to do this every single frame
            let directory = state.directory_trimmed();

            buf.draw_string(
                info_line_area.x as u16,
                info_line_y as u16,
                format!("Dir: {}", directory),
                info_line_area.into(),
                self.theme.text,
            );
            if self.has_focus {
                buf.set_style(info_line_area.into(), self.theme.info_line);
            } else {
                buf.set_style(info_line_area.into(), self.theme.info_line_unfocused);
            }
        }
    }
}

fn get_icon(entry: &DirEntry, file_type: FileType) -> (&'static str, Option<Color>) {
    let ext = entry
        .path
        .extension()
        .map(|s| s.to_string_lossy())
        .unwrap_or(Cow::Borrowed(""));
    let ext: &str = &ext;
    let name = entry.path.file_name().unwrap().to_string_lossy();
    let name: &str = &name;

    if file_type.is_dir() {
        return (
            DIRS.iter()
                .find(|(file_name, _)| *file_name == name)
                .map(|(_, icon)| icon)
                .unwrap_or(&DEFAULT_DIR),
            None,
        );
    } else if file_type.is_symlink() {
        return (LINK, None);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if file_type.is_block_device() {
            return (BLOCK, None);
        }
        if file_type.is_char_device() {
            return (CHAR, None);
        }
        if file_type.is_fifo() {
            return (FIFO, None);
        }
        if file_type.is_socket() {
            return (SOCKET, None);
        }
        if let Some((icon, color)) = FILES.get(name) {
            return (icon, Some(*color));
        }
        if let Some((icon, color)) = EXTS.get(ext) {
            return (icon, Some(*color));
        }
    }
    #[cfg(unix)]
    if is_executable(&entry.metadata) {
        return (DEFAULT_EXE, None);
    }
    (DEFAULT_FILE, None)
}

pub fn is_executable(metadata: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    return std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()) & 0o111 != 0;
    #[cfg(not(unix))]
    false
}

// Copied from yazi dark theme
const DEFAULT_FILE: &str = "";
const DEFAULT_EXE: &str = "";
const DEFAULT_DIR: &str = "";

const LINK: &str = "";
const BLOCK: &str = "";
const CHAR: &str = "";
const FIFO: &str = "";
const SOCKET: &str = "";

const DIRS: &[(&str, &str)] = &[
    (".config", ""),
    (".git", ""),
    (".github", ""),
    (".npm", ""),
    ("Desktop", ""),
    ("Development", ""),
    ("Documents", ""),
    ("Downloads", ""),
    ("Library", ""),
    ("Movies", ""),
    ("Music", ""),
    ("Pictures", ""),
    ("Public", ""),
    ("Videos", ""),
];

static FILES: LazyLock<HashMap<&str, (&str, Color)>> = LazyLock::new(|| {
    FILES_RAW
        .iter()
        .map(|(name, icon, color)| (*name, (*icon, Color::from_str(color).unwrap())))
        .collect()
});

static EXTS: LazyLock<HashMap<&str, (&str, Color)>> = LazyLock::new(|| {
    EXTS_RAW
        .iter()
        .map(|(name, icon, color)| (*name, (*icon, Color::from_str(color).unwrap())))
        .collect()
});

const FILES_RAW: &[(&str, &str, &str)] = &[
    (".babelrc", "", "#cbcb41"),
    (".bash_profile", "", "#89e051"),
    (".bashrc", "", "#89e051"),
    (".clang-format", "", "#6d8086"),
    (".clang-tidy", "", "#6d8086"),
    (".codespellrc", "󰓆", "#35da60"),
    (".condarc", "", "#43b02a"),
    (".dockerignore", "󰡨", "#458ee6"),
    (".ds_store", "", "#41535b"),
    (".editorconfig", "", "#fff2f2"),
    (".env", "", "#faf743"),
    (".eslintignore", "", "#4b32c3"),
    (".eslintrc", "", "#4b32c3"),
    (".git-blame-ignore-revs", "", "#f54d27"),
    (".gitattributes", "", "#f54d27"),
    (".gitconfig", "", "#f54d27"),
    (".gitignore", "", "#f54d27"),
    (".gitlab-ci.yml", "", "#e24329"),
    (".gitmodules", "", "#f54d27"),
    (".gtkrc-2.0", "", "#ffffff"),
    (".gvimrc", "", "#019833"),
    (".justfile", "", "#6d8086"),
    (".luacheckrc", "", "#00a2ff"),
    (".luaurc", "", "#00a2ff"),
    (".mailmap", "󰊢", "#f54d27"),
    (".nanorc", "", "#440077"),
    (".npmignore", "", "#e8274b"),
    (".npmrc", "", "#e8274b"),
    (".nuxtrc", "󱄆", "#00c58e"),
    (".nvmrc", "", "#5fa04e"),
    (".pre-commit-config.yaml", "󰛢", "#f8b424"),
    (".prettierignore", "", "#4285f4"),
    (".prettierrc", "", "#4285f4"),
    (".prettierrc.cjs", "", "#4285f4"),
    (".prettierrc.js", "", "#4285f4"),
    (".prettierrc.json", "", "#4285f4"),
    (".prettierrc.json5", "", "#4285f4"),
    (".prettierrc.mjs", "", "#4285f4"),
    (".prettierrc.toml", "", "#4285f4"),
    (".prettierrc.yaml", "", "#4285f4"),
    (".prettierrc.yml", "", "#4285f4"),
    (".pylintrc", "", "#6d8086"),
    (".settings.json", "", "#854cc7"),
    (".SRCINFO", "󰣇", "#0f94d2"),
    (".vimrc", "", "#019833"),
    (".Xauthority", "", "#e54d18"),
    (".xinitrc", "", "#e54d18"),
    (".Xresources", "", "#e54d18"),
    (".xsession", "", "#e54d18"),
    (".zprofile", "", "#89e051"),
    (".zshenv", "", "#89e051"),
    (".zshrc", "", "#89e051"),
    ("_gvimrc", "", "#019833"),
    ("_vimrc", "", "#019833"),
    ("AUTHORS", "", "#a172ff"),
    ("AUTHORS.txt", "", "#a172ff"),
    ("brewfile", "", "#701516"),
    ("bspwmrc", "", "#2f2f2f"),
    ("build", "", "#89e051"),
    ("build.gradle", "", "#005f87"),
    ("build.zig.zon", "", "#f69a1b"),
    ("bun.lockb", "", "#eadcd1"),
    ("cantorrc", "", "#1c99f3"),
    ("checkhealth", "󰓙", "#75b4fb"),
    ("cmakelists.txt", "", "#dce3eb"),
    ("code_of_conduct", "", "#e41662"),
    ("code_of_conduct.md", "", "#e41662"),
    ("commit_editmsg", "", "#f54d27"),
    ("commitlint.config.js", "󰜘", "#2b9689"),
    ("commitlint.config.ts", "󰜘", "#2b9689"),
    ("compose.yaml", "󰡨", "#458ee6"),
    ("compose.yml", "󰡨", "#458ee6"),
    ("config", "", "#6d8086"),
    ("containerfile", "󰡨", "#458ee6"),
    ("copying", "", "#cbcb41"),
    ("copying.lesser", "", "#cbcb41"),
    ("Directory.Build.props", "", "#00a2ff"),
    ("Directory.Build.targets", "", "#00a2ff"),
    ("Directory.Packages.props", "", "#00a2ff"),
    ("docker-compose.yaml", "󰡨", "#458ee6"),
    ("docker-compose.yml", "󰡨", "#458ee6"),
    ("dockerfile", "󰡨", "#458ee6"),
    ("eslint.config.cjs", "", "#4b32c3"),
    ("eslint.config.js", "", "#4b32c3"),
    ("eslint.config.mjs", "", "#4b32c3"),
    ("eslint.config.ts", "", "#4b32c3"),
    ("ext_typoscript_setup.txt", "", "#ff8700"),
    ("favicon.ico", "", "#cbcb41"),
    ("fp-info-cache", "", "#ffffff"),
    ("fp-lib-table", "", "#ffffff"),
    ("FreeCAD.conf", "", "#cb333b"),
    ("Gemfile", "", "#701516"),
    ("gnumakefile", "", "#6d8086"),
    ("go.mod", "", "#519aba"),
    ("go.sum", "", "#519aba"),
    ("go.work", "", "#519aba"),
    ("gradle-wrapper.properties", "", "#005f87"),
    ("gradle.properties", "", "#005f87"),
    ("gradlew", "", "#005f87"),
    ("groovy", "", "#4a687c"),
    ("gruntfile.babel.js", "", "#e37933"),
    ("gruntfile.coffee", "", "#e37933"),
    ("gruntfile.js", "", "#e37933"),
    ("gruntfile.ts", "", "#e37933"),
    ("gtkrc", "", "#ffffff"),
    ("gulpfile.babel.js", "", "#cc3e44"),
    ("gulpfile.coffee", "", "#cc3e44"),
    ("gulpfile.js", "", "#cc3e44"),
    ("gulpfile.ts", "", "#cc3e44"),
    ("hypridle.conf", "", "#00aaae"),
    ("hyprland.conf", "", "#00aaae"),
    ("hyprlock.conf", "", "#00aaae"),
    ("hyprpaper.conf", "", "#00aaae"),
    ("i18n.config.js", "󰗊", "#7986cb"),
    ("i18n.config.ts", "󰗊", "#7986cb"),
    ("i3blocks.conf", "", "#e8ebee"),
    ("i3status.conf", "", "#e8ebee"),
    ("index.theme", "", "#2db96f"),
    ("ionic.config.json", "", "#4f8ff7"),
    ("justfile", "", "#6d8086"),
    ("kalgebrarc", "", "#1c99f3"),
    ("kdeglobals", "", "#1c99f3"),
    ("kdenlive-layoutsrc", "", "#83b8f2"),
    ("kdenliverc", "", "#83b8f2"),
    ("kritadisplayrc", "", "#f245fb"),
    ("kritarc", "", "#f245fb"),
    ("license", "", "#d0bf41"),
    ("license.md", "", "#d0bf41"),
    ("lxde-rc.xml", "", "#909090"),
    ("lxqt.conf", "", "#0192d3"),
    ("makefile", "", "#6d8086"),
    ("mix.lock", "", "#a074c4"),
    ("mpv.conf", "", "#3b1342"),
    ("node_modules", "", "#e8274b"),
    ("nuxt.config.cjs", "󱄆", "#00c58e"),
    ("nuxt.config.js", "󱄆", "#00c58e"),
    ("nuxt.config.mjs", "󱄆", "#00c58e"),
    ("nuxt.config.ts", "󱄆", "#00c58e"),
    ("package-lock.json", "", "#7a0d21"),
    ("package.json", "", "#e8274b"),
    ("PKGBUILD", "", "#0f94d2"),
    ("platformio.ini", "", "#f6822b"),
    ("pom.xml", "", "#7a0d21"),
    ("prettier.config.cjs", "", "#4285f4"),
    ("prettier.config.js", "", "#4285f4"),
    ("prettier.config.mjs", "", "#4285f4"),
    ("prettier.config.ts", "", "#4285f4"),
    ("procfile", "", "#a074c4"),
    ("PrusaSlicer.ini", "", "#ec6b23"),
    ("PrusaSlicerGcodeViewer.ini", "", "#ec6b23"),
    ("py.typed", "", "#ffbc03"),
    ("QtProject.conf", "", "#40cd52"),
    ("rakefile", "", "#701516"),
    ("readme", "󰂺", "#ededed"),
    ("readme.md", "󰂺", "#ededed"),
    ("rmd", "", "#519aba"),
    ("robots.txt", "󰚩", "#5d7096"),
    ("security", "󰒃", "#bec4c9"),
    ("security.md", "󰒃", "#bec4c9"),
    ("settings.gradle", "", "#005f87"),
    ("svelte.config.js", "", "#ff3e00"),
    ("sxhkdrc", "", "#2f2f2f"),
    ("sym-lib-table", "", "#ffffff"),
    ("tailwind.config.js", "󱏿", "#20c2e3"),
    ("tailwind.config.mjs", "󱏿", "#20c2e3"),
    ("tailwind.config.ts", "󱏿", "#20c2e3"),
    ("tmux.conf", "", "#14ba19"),
    ("tmux.conf.local", "", "#14ba19"),
    ("tsconfig.json", "", "#519aba"),
    ("unlicense", "", "#d0bf41"),
    ("vagrantfile", "", "#1563ff"),
    ("vercel.json", "", "#ffffff"),
    ("vlcrc", "󰕼", "#ee7a00"),
    ("webpack", "󰜫", "#519aba"),
    ("weston.ini", "", "#ffbb01"),
    ("workspace", "", "#89e051"),
    ("xmobarrc", "", "#fd4d5d"),
    ("xmobarrc.hs", "", "#fd4d5d"),
    ("xmonad.hs", "", "#fd4d5d"),
    ("xorg.conf", "", "#e54d18"),
    ("xsettingsd.conf", "", "#e54d18"),
];

const EXTS_RAW: &[(&str, &str, &str)] = &[
    ("3gp", "", "#fd971f"),
    ("3mf", "󰆧", "#888888"),
    ("7z", "", "#eca517"),
    ("a", "", "#dcddd6"),
    ("aac", "", "#00afff"),
    ("adb", "", "#22ffff"),
    ("ads", "", "#ffffff"),
    ("ai", "", "#cbcb41"),
    ("aif", "", "#00afff"),
    ("aiff", "", "#00afff"),
    ("android", "", "#34a853"),
    ("ape", "", "#00afff"),
    ("apk", "", "#34a853"),
    ("apl", "", "#24a148"),
    ("app", "", "#9f0500"),
    ("applescript", "", "#6d8085"),
    ("asc", "󰦝", "#576d7f"),
    ("asm", "", "#0091bd"),
    ("ass", "󰨖", "#ffb713"),
    ("astro", "", "#e23f67"),
    ("avif", "", "#a074c4"),
    ("awk", "", "#4d5a5e"),
    ("azcli", "", "#0078d4"),
    ("bak", "󰁯", "#6d8086"),
    ("bash", "", "#89e051"),
    ("bat", "", "#c1f12e"),
    ("bazel", "", "#89e051"),
    ("bib", "󱉟", "#cbcb41"),
    ("bicep", "", "#519aba"),
    ("bicepparam", "", "#9f74b3"),
    ("bin", "", "#9f0500"),
    ("blade.php", "", "#f05340"),
    ("blend", "󰂫", "#ea7600"),
    ("blp", "󰺾", "#5796e2"),
    ("bmp", "", "#a074c4"),
    ("bqn", "", "#24a148"),
    ("brep", "󰻫", "#839463"),
    ("bz", "", "#eca517"),
    ("bz2", "", "#eca517"),
    ("bz3", "", "#eca517"),
    ("bzl", "", "#89e051"),
    ("c", "", "#599eff"),
    ("c++", "", "#f34b7d"),
    ("cache", "", "#ffffff"),
    ("cast", "", "#fd971f"),
    ("cbl", "", "#005ca5"),
    ("cc", "", "#f34b7d"),
    ("ccm", "", "#f34b7d"),
    ("cfg", "", "#6d8086"),
    ("cjs", "", "#cbcb41"),
    ("clj", "", "#8dc149"),
    ("cljc", "", "#8dc149"),
    ("cljd", "", "#519aba"),
    ("cljs", "", "#519aba"),
    ("cmake", "", "#dce3eb"),
    ("cob", "", "#005ca5"),
    ("cobol", "", "#005ca5"),
    ("coffee", "", "#cbcb41"),
    ("conda", "", "#43b02a"),
    ("conf", "", "#6d8086"),
    ("config.ru", "", "#701516"),
    ("cow", "󰆚", "#965824"),
    ("cp", "", "#519aba"),
    ("cpp", "", "#519aba"),
    ("cppm", "", "#519aba"),
    ("cpy", "", "#005ca5"),
    ("cr", "", "#c8c8c8"),
    ("crdownload", "", "#44cda8"),
    ("cs", "󰌛", "#596706"),
    ("csh", "", "#4d5a5e"),
    ("cshtml", "󱦗", "#512bd4"),
    ("cson", "", "#cbcb41"),
    ("csproj", "󰪮", "#512bd4"),
    ("css", "", "#42a5f5"),
    ("csv", "", "#89e051"),
    ("cts", "", "#519aba"),
    ("cu", "", "#89e051"),
    ("cue", "󰲹", "#ed95ae"),
    ("cuh", "", "#a074c4"),
    ("cxx", "", "#519aba"),
    ("cxxm", "", "#519aba"),
    ("d", "", "#b03931"),
    ("d.ts", "", "#d59855"),
    ("dart", "", "#03589c"),
    ("db", "", "#dad8d8"),
    ("dconf", "", "#ffffff"),
    ("desktop", "", "#563d7c"),
    ("diff", "", "#41535b"),
    ("dll", "", "#4d2c0b"),
    ("doc", "󰈬", "#185abd"),
    ("Dockerfile", "󰡨", "#458ee6"),
    ("docx", "󰈬", "#185abd"),
    ("dot", "󱁉", "#30638e"),
    ("download", "", "#44cda8"),
    ("drl", "", "#ffafaf"),
    ("dropbox", "", "#0061fe"),
    ("dump", "", "#dad8d8"),
    ("dwg", "󰻫", "#839463"),
    ("dxf", "󰻫", "#839463"),
    ("ebook", "", "#eab16d"),
    ("ebuild", "", "#4c416e"),
    ("edn", "", "#519aba"),
    ("eex", "", "#a074c4"),
    ("ejs", "", "#cbcb41"),
    ("el", "", "#8172be"),
    ("elc", "", "#8172be"),
    ("elf", "", "#9f0500"),
    ("elm", "", "#519aba"),
    ("eln", "", "#8172be"),
    ("env", "", "#faf743"),
    ("eot", "", "#ececec"),
    ("epp", "", "#ffa61a"),
    ("epub", "", "#eab16d"),
    ("erb", "", "#701516"),
    ("erl", "", "#b83998"),
    ("ex", "", "#a074c4"),
    ("exe", "", "#9f0500"),
    ("exs", "", "#a074c4"),
    ("f#", "", "#519aba"),
    ("f3d", "󰻫", "#839463"),
    ("f90", "󱈚", "#734f96"),
    ("fbx", "󰆧", "#888888"),
    ("fcbak", "", "#cb333b"),
    ("fcmacro", "", "#cb333b"),
    ("fcmat", "", "#cb333b"),
    ("fcparam", "", "#cb333b"),
    ("fcscript", "", "#cb333b"),
    ("fcstd", "", "#cb333b"),
    ("fcstd1", "", "#cb333b"),
    ("fctb", "", "#cb333b"),
    ("fctl", "", "#cb333b"),
    ("fdmdownload", "", "#44cda8"),
    ("fish", "", "#4d5a5e"),
    ("flac", "", "#0075aa"),
    ("flc", "", "#ececec"),
    ("flf", "", "#ececec"),
    ("fnl", "", "#fff3d7"),
    ("fodg", "", "#fffb57"),
    ("fodp", "", "#fe9c45"),
    ("fods", "", "#78fc4e"),
    ("fodt", "", "#2dcbfd"),
    ("fs", "", "#519aba"),
    ("fsi", "", "#519aba"),
    ("fsscript", "", "#519aba"),
    ("fsx", "", "#519aba"),
    ("gcode", "󰐫", "#1471ad"),
    ("gd", "", "#6d8086"),
    ("gemspec", "", "#701516"),
    ("gif", "", "#a074c4"),
    ("git", "", "#f14c28"),
    ("glb", "", "#ffb13b"),
    ("gleam", "", "#ffaff3"),
    ("gnumakefile", "", "#6d8086"),
    ("go", "", "#519aba"),
    ("godot", "", "#6d8086"),
    ("gpr", "", "#ff33ff"),
    ("gql", "", "#e535ab"),
    ("gradle", "", "#005f87"),
    ("graphql", "", "#e535ab"),
    ("gresource", "", "#ffffff"),
    ("gv", "󱁉", "#30638e"),
    ("gz", "", "#eca517"),
    ("h", "", "#a074c4"),
    ("haml", "", "#eaeae1"),
    ("hbs", "", "#f0772b"),
    ("heex", "", "#a074c4"),
    ("hex", "", "#2e63ff"),
    ("hh", "", "#a074c4"),
    ("hpp", "", "#a074c4"),
    ("hrl", "", "#b83998"),
    ("hs", "", "#a074c4"),
    ("htm", "", "#e34c26"),
    ("html", "", "#e44d26"),
    ("http", "", "#008ec7"),
    ("huff", "󰡘", "#4242c7"),
    ("hurl", "", "#ff0288"),
    ("hx", "", "#ea8220"),
    ("hxx", "", "#a074c4"),
    ("ical", "", "#2b2e83"),
    ("icalendar", "", "#2b2e83"),
    ("ico", "", "#cbcb41"),
    ("ics", "", "#2b2e83"),
    ("ifb", "", "#2b2e83"),
    ("ifc", "󰻫", "#839463"),
    ("ige", "󰻫", "#839463"),
    ("iges", "󰻫", "#839463"),
    ("igs", "󰻫", "#839463"),
    ("image", "", "#d0bec8"),
    ("img", "", "#d0bec8"),
    ("import", "", "#ececec"),
    ("info", "", "#ffffcd"),
    ("ini", "", "#6d8086"),
    ("ino", "", "#56b6c2"),
    ("ipynb", "", "#f57d01"),
    ("iso", "", "#d0bec8"),
    ("ixx", "", "#519aba"),
    ("java", "", "#cc3e44"),
    ("jl", "", "#a270ba"),
    ("jpeg", "", "#a074c4"),
    ("jpg", "", "#a074c4"),
    ("js", "", "#cbcb41"),
    ("json", "", "#cbcb41"),
    ("json5", "", "#cbcb41"),
    ("jsonc", "", "#cbcb41"),
    ("jsx", "", "#20c2e3"),
    ("jwmrc", "", "#0078cd"),
    ("jxl", "", "#a074c4"),
    ("kbx", "󰯄", "#737672"),
    ("kdb", "", "#529b34"),
    ("kdbx", "", "#529b34"),
    ("kdenlive", "", "#83b8f2"),
    ("kdenlivetitle", "", "#83b8f2"),
    ("kicad_dru", "", "#ffffff"),
    ("kicad_mod", "", "#ffffff"),
    ("kicad_pcb", "", "#ffffff"),
    ("kicad_prl", "", "#ffffff"),
    ("kicad_pro", "", "#ffffff"),
    ("kicad_sch", "", "#ffffff"),
    ("kicad_sym", "", "#ffffff"),
    ("kicad_wks", "", "#ffffff"),
    ("ko", "", "#dcddd6"),
    ("kpp", "", "#f245fb"),
    ("kra", "", "#f245fb"),
    ("krz", "", "#f245fb"),
    ("ksh", "", "#4d5a5e"),
    ("kt", "", "#7f52ff"),
    ("kts", "", "#7f52ff"),
    ("lck", "", "#bbbbbb"),
    ("leex", "", "#a074c4"),
    ("less", "", "#563d7c"),
    ("lff", "", "#ececec"),
    ("lhs", "", "#a074c4"),
    ("lib", "", "#4d2c0b"),
    ("license", "", "#cbcb41"),
    ("liquid", "", "#95bf47"),
    ("lock", "", "#bbbbbb"),
    ("log", "󰌱", "#dddddd"),
    ("lrc", "󰨖", "#ffb713"),
    ("lua", "", "#51a0cf"),
    ("luac", "", "#51a0cf"),
    ("luau", "", "#00a2ff"),
    ("m", "", "#599eff"),
    ("m3u", "󰲹", "#ed95ae"),
    ("m3u8", "󰲹", "#ed95ae"),
    ("m4a", "", "#00afff"),
    ("m4v", "", "#fd971f"),
    ("magnet", "", "#a51b16"),
    ("makefile", "", "#6d8086"),
    ("markdown", "", "#dddddd"),
    ("material", "", "#b83998"),
    ("md", "", "#dddddd"),
    ("md5", "󰕥", "#8c86af"),
    ("mdx", "", "#519aba"),
    ("mint", "󰌪", "#87c095"),
    ("mjs", "", "#f1e05a"),
    ("mk", "", "#6d8086"),
    ("mkv", "", "#fd971f"),
    ("ml", "", "#e37933"),
    ("mli", "", "#e37933"),
    ("mm", "", "#519aba"),
    ("mo", "", "#9772fb"),
    ("mobi", "", "#eab16d"),
    ("mojo", "", "#ff4c1f"),
    ("mov", "", "#fd971f"),
    ("mp3", "", "#00afff"),
    ("mp4", "", "#fd971f"),
    ("mpp", "", "#519aba"),
    ("msf", "", "#137be1"),
    ("mts", "", "#519aba"),
    ("mustache", "", "#e37933"),
    ("nfo", "", "#ffffcd"),
    ("nim", "", "#f3d400"),
    ("nix", "", "#7ebae4"),
    ("norg", "", "#4878be"),
    ("nswag", "", "#85ea2d"),
    ("nu", "", "#3aa675"),
    ("o", "", "#9f0500"),
    ("obj", "󰆧", "#888888"),
    ("odf", "", "#ff5a96"),
    ("odg", "", "#fffb57"),
    ("odin", "󰟢", "#3882d2"),
    ("odp", "", "#fe9c45"),
    ("ods", "", "#78fc4e"),
    ("odt", "", "#2dcbfd"),
    ("oga", "", "#0075aa"),
    ("ogg", "", "#0075aa"),
    ("ogv", "", "#fd971f"),
    ("ogx", "", "#fd971f"),
    ("opus", "", "#0075aa"),
    ("org", "", "#77aa99"),
    ("otf", "", "#ececec"),
    ("out", "", "#9f0500"),
    ("part", "", "#44cda8"),
    ("patch", "", "#41535b"),
    ("pck", "", "#6d8086"),
    ("pcm", "", "#0075aa"),
    ("pdf", "", "#b30b00"),
    ("php", "", "#a074c4"),
    ("pl", "", "#519aba"),
    ("pls", "󰲹", "#ed95ae"),
    ("ply", "󰆧", "#888888"),
    ("pm", "", "#519aba"),
    ("png", "", "#a074c4"),
    ("po", "", "#2596be"),
    ("pot", "", "#2596be"),
    ("pp", "", "#ffa61a"),
    ("ppt", "󰈧", "#cb4a32"),
    ("pptx", "󰈧", "#cb4a32"),
    ("prisma", "", "#5a67d8"),
    ("pro", "", "#e4b854"),
    ("ps1", "󰨊", "#4273ca"),
    ("psb", "", "#519aba"),
    ("psd", "", "#519aba"),
    ("psd1", "󰨊", "#6975c4"),
    ("psm1", "󰨊", "#6975c4"),
    ("pub", "󰷖", "#e3c58e"),
    ("pxd", "", "#5aa7e4"),
    ("pxi", "", "#5aa7e4"),
    ("py", "", "#ffbc03"),
    ("pyc", "", "#ffe291"),
    ("pyd", "", "#ffe291"),
    ("pyi", "", "#ffbc03"),
    ("pyo", "", "#ffe291"),
    ("pyw", "", "#5aa7e4"),
    ("pyx", "", "#5aa7e4"),
    ("qm", "", "#2596be"),
    ("qml", "", "#40cd52"),
    ("qrc", "", "#40cd52"),
    ("qss", "", "#40cd52"),
    ("query", "", "#90a850"),
    ("r", "󰟔", "#2266ba"),
    ("R", "󰟔", "#2266ba"),
    ("rake", "", "#701516"),
    ("rar", "", "#eca517"),
    ("razor", "󱦘", "#512bd4"),
    ("rb", "", "#701516"),
    ("res", "", "#cc3e44"),
    ("resi", "", "#f55385"),
    ("rlib", "", "#dea584"),
    ("rmd", "", "#519aba"),
    ("rproj", "󰗆", "#358a5b"),
    ("rs", "", "#dea584"),
    ("rss", "", "#fb9d3b"),
    ("s", "", "#0071c5"),
    ("sass", "", "#f55385"),
    ("sbt", "", "#cc3e44"),
    ("sc", "", "#cc3e44"),
    ("scad", "", "#f9d72c"),
    ("scala", "", "#cc3e44"),
    ("scm", "󰘧", "#eeeeee"),
    ("scss", "", "#f55385"),
    ("sh", "", "#4d5a5e"),
    ("sha1", "󰕥", "#8c86af"),
    ("sha224", "󰕥", "#8c86af"),
    ("sha256", "󰕥", "#8c86af"),
    ("sha384", "󰕥", "#8c86af"),
    ("sha512", "󰕥", "#8c86af"),
    ("sig", "󰘧", "#e37933"),
    ("signature", "󰘧", "#e37933"),
    ("skp", "󰻫", "#839463"),
    ("sldasm", "󰻫", "#839463"),
    ("sldprt", "󰻫", "#839463"),
    ("slim", "", "#e34c26"),
    ("sln", "", "#854cc7"),
    ("slnx", "", "#854cc7"),
    ("slvs", "󰻫", "#839463"),
    ("sml", "󰘧", "#e37933"),
    ("so", "", "#dcddd6"),
    ("sol", "", "#519aba"),
    ("spec.js", "", "#cbcb41"),
    ("spec.jsx", "", "#20c2e3"),
    ("spec.ts", "", "#519aba"),
    ("spec.tsx", "", "#1354bf"),
    ("spx", "", "#0075aa"),
    ("sql", "", "#dad8d8"),
    ("sqlite", "", "#dad8d8"),
    ("sqlite3", "", "#dad8d8"),
    ("srt", "󰨖", "#ffb713"),
    ("ssa", "󰨖", "#ffb713"),
    ("ste", "󰻫", "#839463"),
    ("step", "󰻫", "#839463"),
    ("stl", "󰆧", "#888888"),
    ("stp", "󰻫", "#839463"),
    ("strings", "", "#2596be"),
    ("styl", "", "#8dc149"),
    ("sub", "󰨖", "#ffb713"),
    ("sublime", "", "#e37933"),
    ("suo", "", "#854cc7"),
    ("sv", "󰍛", "#019833"),
    ("svelte", "", "#ff3e00"),
    ("svg", "󰜡", "#ffb13b"),
    ("svh", "󰍛", "#019833"),
    ("swift", "", "#e37933"),
    ("t", "", "#519aba"),
    ("tbc", "󰛓", "#1e5cb3"),
    ("tcl", "󰛓", "#1e5cb3"),
    ("templ", "", "#dbbd30"),
    ("terminal", "", "#31b53e"),
    ("test.js", "", "#cbcb41"),
    ("test.jsx", "", "#20c2e3"),
    ("test.ts", "", "#519aba"),
    ("test.tsx", "", "#1354bf"),
    ("tex", "", "#3d6117"),
    ("tf", "", "#5f43e9"),
    ("tfvars", "", "#5f43e9"),
    ("tgz", "", "#eca517"),
    ("tmux", "", "#14ba19"),
    ("toml", "", "#9c4221"),
    ("torrent", "", "#44cda8"),
    ("tres", "", "#6d8086"),
    ("ts", "", "#519aba"),
    ("tscn", "", "#6d8086"),
    ("tsconfig", "", "#ff8700"),
    ("tsx", "", "#1354bf"),
    ("ttf", "", "#ececec"),
    ("twig", "", "#8dc149"),
    ("txt", "󰈙", "#89e051"),
    ("txz", "", "#eca517"),
    ("typ", "", "#0dbcc0"),
    ("typoscript", "", "#ff8700"),
    ("ui", "", "#015bf0"),
    ("v", "󰍛", "#019833"),
    ("vala", "", "#7b3db9"),
    ("vh", "󰍛", "#019833"),
    ("vhd", "󰍛", "#019833"),
    ("vhdl", "󰍛", "#019833"),
    ("vi", "", "#fec60a"),
    ("vim", "", "#019833"),
    ("vsh", "", "#5d87bf"),
    ("vsix", "", "#854cc7"),
    ("vue", "", "#8dc149"),
    ("wasm", "", "#5c4cdb"),
    ("wav", "", "#00afff"),
    ("webm", "", "#fd971f"),
    ("webmanifest", "", "#f1e05a"),
    ("webp", "", "#a074c4"),
    ("webpack", "󰜫", "#519aba"),
    ("wma", "", "#00afff"),
    ("woff", "", "#ececec"),
    ("woff2", "", "#ececec"),
    ("wrl", "󰆧", "#888888"),
    ("wrz", "󰆧", "#888888"),
    ("wv", "", "#00afff"),
    ("wvc", "", "#00afff"),
    ("x", "", "#599eff"),
    ("xaml", "󰙳", "#512bd4"),
    ("xcf", "", "#635b46"),
    ("xcplayground", "", "#e37933"),
    ("xcstrings", "", "#2596be"),
    ("xls", "󰈛", "#207245"),
    ("xlsx", "󰈛", "#207245"),
    ("xm", "", "#519aba"),
    ("xml", "󰗀", "#e37933"),
    ("xpi", "", "#ff1b01"),
    ("xul", "", "#e37933"),
    ("xz", "", "#eca517"),
    ("yaml", "", "#6d8086"),
    ("yml", "", "#6d8086"),
    ("zig", "", "#f69a1b"),
    ("zip", "", "#eca517"),
    ("zsh", "", "#89e051"),
    ("zst", "", "#eca517"),
    ("🔥", "", "#ff4c1f"),
];
