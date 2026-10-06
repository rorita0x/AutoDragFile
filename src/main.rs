mod config;

use clap::Parser;
use config::{Config, DEFAULT_CONFIG};
use gtk::cairo::{RectangleInt, Region};
use gtk::prelude::*;
use gtk::{gdk, glib, CssProvider, EventBox, Fixed, Label, TargetList, Window, WindowType};
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;
use std::time::Duration;

#[derive(Parser)]
#[command(
    version,
    about = "Spawns a small box under the cursor from which FILE can be dragged"
)]
struct Cli {
    #[arg(
        short,
        long,
        value_name = "PATH",
        help = "TOML file overriding texts, colours and timings"
    )]
    config: Option<PathBuf>,

    #[arg(
        long,
        exclusive = true,
        help = "Print the default config to stdout and exit"
    )]
    print_default_config: bool,

    #[arg(required_unless_present = "print_default_config")]
    file: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    if cli.print_default_config {
        print!("{DEFAULT_CONFIG}");
        return;
    }

    let config = Config::load(cli.config.as_deref()).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    let filepath = cli.file.expect("clap enforces FILE");
    if !filepath.is_file() {
        eprintln!(
            "{}",
            config.text.file_not_found.replace("{path}", &filepath.display().to_string())
        );
        process::exit(1);
    }

    if gtk::init().is_err() {
        eprintln!("{}", config.text.gtk_init_failed);
        process::exit(1);
    }

    let css = CssProvider::new();
    if let Err(e) = css.load_from_data(config.css().as_bytes()) {
        eprintln!("Invalid style config: {e}");
        process::exit(1);
    }
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().expect("GTK is initialised"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let drag_box = build_drag_box(&config);
    let drag_started = Rc::new(Cell::new(false));
    setup_drag_source(&drag_box, &filepath, drag_started.clone(), &config);

    let window = if gtk_layer_shell::is_supported() {
        build_layer_overlay(&config, &drag_box)
    } else {
        build_x11_window(&config, &drag_box)
    };
    window.connect_destroy(|_| gtk::main_quit());

    let timeout_reached = config.text.timeout_reached.clone();
    glib::timeout_add_local_once(Duration::from_millis(config.behavior.timeout_ms), move || {
        if !drag_started.get() {
            println!("{timeout_reached}");
            gtk::main_quit();
        }
    });

    gtk::main();
}

fn build_drag_box(config: &Config) -> EventBox {
    let label = Label::new(None);
    label.set_markup(&config.text.label);

    let drag_box = EventBox::new();
    drag_box.style_context().add_class("drag-box");
    drag_box.set_size_request(config.style.width, config.style.height);
    drag_box.add(&label);
    drag_box
}

fn setup_drag_source(
    drag_box: &EventBox,
    filepath: &Path,
    drag_started: Rc<Cell<bool>>,
    config: &Config,
) {
    let targets = TargetList::new(&[]);
    targets.add_uri_targets(0);
    drag_box.connect_button_press_event(move |drag_box, press| {
        if press.button() != 1 {
            return glib::Propagation::Proceed;
        }
        let (x, y) = press.position();
        drag_box.drag_begin_with_coordinates(
            &targets,
            gdk::DragAction::COPY,
            1,
            Some(press),
            x as i32,
            y as i32,
        );
        glib::Propagation::Stop
    });

    let filepath = filepath.to_path_buf();
    drag_box.connect_drag_data_get(move |_, _, data, _, _| {
        if let Ok(uri) = filepath
            .canonicalize()
            .map_err(|_| ())
            .and_then(|abs| glib::filename_to_uri(&abs, None).map_err(|_| ()))
        {
            data.set_uris(&[&uri]);
        }
    });

    drag_box.connect_drag_begin(move |_, _| drag_started.set(true));

    let drag_finished = config.text.drag_finished.clone();
    drag_box.connect_drag_end(move |_, _| {
        println!("{drag_finished}");
        gtk::main_quit();
    });
}

fn new_transparent_window(config: &Config) -> Window {
    let window = Window::new(WindowType::Toplevel);
    window.set_title(&config.text.window_title);
    window.set_app_paintable(true);
    window.style_context().add_class("transparent");
    if let Some(visual) = WidgetExt::screen(&window).and_then(|s| s.rgba_visual()) {
        window.set_visual(Some(&visual));
    }
    window
}

fn build_layer_overlay(config: &Config, drag_box: &EventBox) -> Window {
    let window = new_transparent_window(config);
    window.init_layer_shell();
    window.set_namespace("autodragfile");
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_exclusive_zone(-1);
    for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
        window.set_anchor(edge, true);
    }

    let fixed = Fixed::new();
    fixed.put(drag_box, 0, 0);
    window.add(&fixed);
    window.add_events(gdk::EventMask::POINTER_MOTION_MASK | gdk::EventMask::ENTER_NOTIFY_MASK);

    let placed = Rc::new(Cell::new(false));
    let place_at_cursor = {
        let window = window.clone();
        let drag_box = drag_box.clone();
        let (offset_x, offset_y) = (config.behavior.offset_x, config.behavior.offset_y);
        move |(cursor_x, cursor_y): (f64, f64)| {
            if placed.replace(true) {
                return;
            }
            drag_box.show();
            let (_, natural) = drag_box.preferred_size();
            let x = (cursor_x as i32 + offset_x).clamp(0, (window.allocated_width() - natural.width).max(0));
            let y = (cursor_y as i32 + offset_y).clamp(0, (window.allocated_height() - natural.height).max(0));
            fixed.move_(&drag_box, x, y);
            window.input_shape_combine_region(Some(&Region::create_rectangle(&RectangleInt::new(
                x,
                y,
                natural.width,
                natural.height,
            ))));
        }
    };
    let place_on_motion = place_at_cursor.clone();
    window.connect_enter_notify_event(move |_, event| {
        place_at_cursor(event.position());
        glib::Propagation::Proceed
    });
    window.connect_motion_notify_event(move |_, event| {
        place_on_motion(event.position());
        glib::Propagation::Proceed
    });

    let win = window.clone();
    drag_box.connect_drag_begin(move |drag_box, _| {
        drag_box.set_opacity(0.0);
        win.input_shape_combine_region(Some(&Region::create()));
    });

    window.show_all();
    drag_box.hide();
    window
}

fn build_x11_window(config: &Config, drag_box: &EventBox) -> Window {
    let window = new_transparent_window(config);
    window.set_decorated(false);
    window.set_keep_above(true);
    window.set_resizable(false);
    window.add(drag_box);

    let pointer = gdk::Display::default()
        .and_then(|d| d.default_seat())
        .and_then(|s| s.pointer());
    if let Some(pointer) = pointer {
        let (_, x, y) = pointer.position();
        window.move_(x + config.behavior.offset_x, y + config.behavior.offset_y);
    }

    drag_box.connect_drag_begin(move |drag_box, _| {
        if let Some(toplevel) = drag_box.toplevel() {
            toplevel.hide();
        }
    });

    window.show_all();
    window
}
