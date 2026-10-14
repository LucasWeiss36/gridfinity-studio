//! App workflows exercised in an in-memory egui context, without native windows.
//! Test projects live only below the build directory; user settings are not loaded.
use crate::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

#[test]
fn design_two_home_keeps_create_action_visible_at_supported_window_sizes() {
    for mode in [theme::Mode::Light, theme::Mode::Dark] {
        for size in [Vec2::new(1100.0, 740.0), Vec2::new(1440.0, 940.0)] {
            let fixture = Fixture::new();
            let ctx = egui::Context::default();
            theme::apply(&ctx, mode);
            let mut app = fixture.studio(&ctx);
            app.show_home = true;
            let mut output = None;
            for _ in 0..3 {
                output = Some(ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| {
                        app.reference_header(ctx);
                        app.atelier_rail(ctx);
                        app.atelier_home(ctx);
                    },
                ));
            }
            let output = output.unwrap();
            let point = text_position(&output, "Kasten erstellen   →").expect("create button");
            assert!(
                point.x > 90.0
                    && point.x < size.x - 20.0
                    && point.y > 80.0
                    && point.y < size.y - 20.0,
                "{point:?} outside {size:?}"
            );
            for label in ["Hell", "Dunkel"] {
                let point = text_position(&output, label).expect("theme switch");
                assert!(point.x > 0.0 && point.x < 90.0 && point.y > 500.0 && point.y < size.y);
            }
        }
    }
}

#[test]
fn printer_preview_reacts_to_custom_dimensions_without_changing_the_design() {
    let fixture = Fixture::new();
    let ctx = egui::Context::default();
    let mut app = fixture.studio(&ctx);
    let original = app.key();
    app.printer_panel.settings = printer::Settings {
        profile_id: Some("custom".into()),
        custom_size: [100.0, 100.0, 100.0],
        margin: 0.0,
    };
    let frame = |app: &mut Studio| {
        ctx.run(input(vec![]), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| app.viewport(ui));
        })
    };
    frame(&mut app);
    let output = frame(&mut app);
    assert!(text_position(&output, "Druckbett").is_some());
    assert!(text_position(&output, "Dieses Teil passt in den Bauraum.").is_some());
    app.printer_panel.settings.custom_size[0] = 60.0;
    let output = frame(&mut app);
    assert!(
        text_position(
            &output,
            "Das Teil ragt über die nutzbare Druckfläche hinaus."
        )
        .is_some()
    );
    app.cfg.grid_x = 3; // The old mesh remains visible until the background build completes.
    let output = frame(&mut app);
    assert!(text_position(&output, "Vorschau wird aktualisiert …").is_some());
    assert!(text_position(&output, "Dieses Teil passt in den Bauraum.").is_none());
    app.cfg.grid_x = 2;
    app.printer_bed_view = false;
    let output = frame(&mut app);
    assert!(
        text_position(
            &output,
            "Das Teil ragt über die nutzbare Druckfläche hinaus."
        )
        .is_none()
    );
    app.printer_panel.settings.profile_id = None;
    let output = frame(&mut app);
    assert!(text_position(&output, "Druckbett").is_none());
    assert_eq!(app.key(), original);
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("ui-tests");
        fs::create_dir_all(&parent).unwrap();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = parent.join(format!(
            "{}-{nonce}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }

    fn studio(&self, ctx: &egui::Context) -> Studio {
        let cfg = Design::default();
        let key = serde_json::to_string(&cfg).unwrap();
        Studio {
            root: self.root.clone(),
            parts: model::generate(&cfg).unwrap(),
            cfg,
            task: None,
            observed: key.clone(),
            built: key.clone(),
            changed: Instant::now(),
            error: None,
            operation_error: None,
            pending_transition: None,
            allow_close: false,
            message: String::new(),
            active: 0,
            assembly: true,
            explode: 0.5,
            yaw: -0.38,
            pitch: 0.84,
            zoom: 0.88,
            pan: Vec2::ZERO,
            view_plan: false,
            selection: None,
            tool: PlanTool::Select,
            wall_start: None,
            wall_editor: wall_editor::Editor::default(),
            wall_drag: None,
            dimension_edit: None,
            snap: 1.0,
            show_plan_grid: false,
            show_measurements: true,
            show_home: false,
            home_product: Product::Bin,
            appearance: theme::Appearance::default(),
            sidebar_visible: true,
            help_visible: false,
            show_inches: false,
            show_build_plate: true,
            printer_bed_view: true,
            export_dialog: false,
            saved_path: None,
            saved_key: key,
            library_browser: library_ui::Browser::for_test(self.root.join("library")),
            last_duration: Duration::ZERO,
            gpu: None,
            thumbnails: thumbnails::Thumbnails::new(None, ctx.clone()),
            printer_panel: printer_ui::Panel::for_test(),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // `root` is uniquely created above and never changed by the tested app.
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn dirty_built_design(app: &mut Studio) -> String {
    app.cfg.title = "Ungespeicherter Entwurf".into();
    app.cfg.grid_x = 3;
    app.parts = model::generate(&app.cfg).unwrap();
    app.built = app.key();
    app.observed = app.built.clone();
    assert!(app.fresh());
    assert_ne!(app.key(), app.saved_key);
    app.key()
}

fn input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            Vec2::new(1000.0, 700.0),
        )),
        events,
        ..Default::default()
    }
}

fn text_position(output: &egui::FullOutput, expected: &str) -> Option<Pos2> {
    fn search(shape: &egui::Shape, expected: &str) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(text) if text.galley.job.text == expected => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| search(shape, expected)),
            _ => None,
        }
    }
    output
        .shapes
        .iter()
        .find_map(|shape| search(&shape.shape, expected))
}

fn transition_frame(app: &mut Studio, ctx: &egui::Context) -> egui::FullOutput {
    ctx.run(input(vec![]), |ctx| app.transition_dialog(ctx))
}

fn click_transition(app: &mut Studio, ctx: &egui::Context, label: &str) -> egui::FullOutput {
    // First pass establishes area placement; the next pass provides stable hit targets.
    transition_frame(app, ctx);
    let output = transition_frame(app, ctx);
    let position = text_position(&output, label)
        .unwrap_or_else(|| panic!("Dialogschaltfläche fehlt: {label}"));
    let mut output = output;
    for pressed in [true, false] {
        output = ctx.run(
            input(vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]),
            |ctx| app.transition_dialog(ctx),
        );
    }
    output
}

fn command_present(
    output: &egui::FullOutput,
    predicate: impl Fn(&egui::ViewportCommand) -> bool,
) -> bool {
    output
        .viewport_output
        .values()
        .any(|viewport| viewport.commands.iter().any(&predicate))
}

#[test]
fn operation_errors_stay_visible_without_invalidating_a_current_model() {
    let fixture = Fixture::new();
    let ctx = egui::Context::default();
    let mut app = fixture.studio(&ctx);
    let before = app.key();
    app.operation_error = Some("Test: Zielordner ist nicht beschreibbar.".into());
    app.poll(&ctx);
    assert_eq!(app.key(), before);
    assert!(app.fresh());
    assert!(app.error.is_none());
    assert!(app.dialog_active());
    let _ = ctx.run(input(vec![]), |ctx| app.operation_error_dialog(ctx));
    let output = ctx.run(input(vec![]), |ctx| app.operation_error_dialog(ctx));
    assert!(text_position(&output, "Vorgang fehlgeschlagen").is_some());
    assert!(text_position(&output, app.operation_error.as_deref().unwrap()).is_some());
    assert!(app.fresh());
}

#[test]
fn invalid_external_files_preserve_the_current_project_and_show_an_error() {
    let fixture = Fixture::new();
    let ctx = egui::Context::default();
    let mut app = fixture.studio(&ctx);
    let before = dirty_built_design(&mut app);
    let original_saved_key = app.saved_key.clone();
    let path = fixture.root.join("invalid.gridfinity.json");
    let invalid_design = Design {
        grid_x: 0,
        ..Design::default()
    };
    for bytes in [
        b"not json".to_vec(),
        serde_json::to_vec(&invalid_design).unwrap(),
    ] {
        fs::write(&path, bytes).unwrap();
        app.load_path(path.clone());
        app.poll(&ctx);
        assert_eq!(app.key(), before);
        assert_eq!(app.saved_key, original_saved_key);
        assert!(app.saved_path.is_none());
        assert!(app.fresh());
        assert!(app.error.is_none());
        assert!(app.pending_transition.is_none());
        let message = app.operation_error.as_deref().unwrap();
        assert!(message.starts_with("Projektdatei konnte nicht geöffnet werden:"));
        let _ = ctx.run(input(vec![]), |ctx| app.operation_error_dialog(ctx));
        let output = ctx.run(input(vec![]), |ctx| app.operation_error_dialog(ctx));
        assert!(text_position(&output, "Vorgang fehlgeschlagen").is_some());
    }
}

#[test]
fn replacing_a_dirty_design_requires_a_decision_and_cancel_keeps_the_model() {
    let fixture = Fixture::new();
    let ctx = egui::Context::default();
    let mut app = fixture.studio(&ctx);
    let before = dirty_built_design(&mut app);
    let replacement = Design {
        title: "Neues Projekt".into(),
        grid_y: 4,
        ..Design::default()
    };
    let replacement_key = serde_json::to_string(&replacement).unwrap();
    let path = fixture.root.join("new.gridfinity.json");
    app.request_replace(replacement.clone(), Some(path.clone()));
    assert_eq!(app.key(), before);
    assert!(app.pending_transition.is_some());
    assert!(app.fresh());
    // An unrelated completion notification must not skip the confirmation stage.
    app.pending_save_finished();
    transition_frame(&mut app, &ctx);
    assert_eq!(app.key(), before);
    assert!(app.pending_transition.is_some());
    click_transition(&mut app, &ctx, "Abbrechen");
    assert!(app.pending_transition.is_none());
    assert_eq!(app.key(), before);
    assert!(app.fresh());
    app.request_replace(replacement, Some(path.clone()));
    click_transition(&mut app, &ctx, "Änderungen verwerfen");
    assert!(app.pending_transition.is_none());
    assert_eq!(app.key(), replacement_key);
    assert_eq!(app.saved_key, replacement_key);
    assert_eq!(app.saved_path, Some(path));
    assert!(app.parts.is_empty());
    assert!(
        !app.fresh(),
        "Die Vorschau des alten Projekts darf nicht exportierbar bleiben."
    );
}

#[test]
fn opening_an_example_and_cancelling_its_save_keeps_unsaved_work() {
    let fixture = Fixture::new();
    let ctx = egui::Context::default();
    let mut app = fixture.studio(&ctx);
    let before = dirty_built_design(&mut app);
    app.load_example(include_str!("../examples/baseplate-clips.gridfinity.json"));
    assert_eq!(app.key(), before);
    assert!(app.pending_transition.is_some());
    click_transition(&mut app, &ctx, "Speichern …");
    assert!(app.library_browser.visible);
    assert!(app.pending_transition.is_some());
    assert_eq!(app.key(), before);
    // Closing the project save browser cancels the requested replacement.
    app.library_browser.visible = false;
    transition_frame(&mut app, &ctx);
    assert!(app.pending_transition.is_none());
    assert_eq!(app.key(), before);
    assert!(app.fresh());
    assert_ne!(app.key(), app.saved_key);
}

#[test]
fn close_requests_are_cancelled_until_unsaved_work_is_explicitly_discarded() {
    let fixture = Fixture::new();
    let ctx = egui::Context::default();
    let mut app = fixture.studio(&ctx);
    let mut close_input = input(vec![]);
    close_input
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(egui::ViewportEvent::Close);
    let clean = ctx.run(close_input.clone(), |ctx| app.handle_close_request(ctx));
    assert!(!command_present(&clean, |command| matches!(
        command,
        egui::ViewportCommand::CancelClose
    )));
    assert!(app.pending_transition.is_none());
    let before = dirty_built_design(&mut app);
    let dirty = ctx.run(close_input.clone(), |ctx| app.handle_close_request(ctx));
    assert!(command_present(&dirty, |command| matches!(
        command,
        egui::ViewportCommand::CancelClose
    )));
    assert!(app.pending_transition.is_some());
    assert!(!app.allow_close);
    click_transition(&mut app, &ctx, "Abbrechen");
    assert!(app.pending_transition.is_none());
    assert_eq!(app.key(), before);
    assert!(!app.allow_close);
    let _ = ctx.run(close_input, |ctx| app.handle_close_request(ctx));
    let confirmed = click_transition(&mut app, &ctx, "Ohne Speichern schließen");
    assert!(command_present(&confirmed, |command| matches!(
        command,
        egui::ViewportCommand::Close
    )));
    assert!(app.allow_close);
    assert!(app.pending_transition.is_none());
}

#[test]
fn an_unchanged_project_can_open_a_valid_external_file_without_a_prompt() {
    let fixture = Fixture::new();
    let ctx = egui::Context::default();
    let mut app = fixture.studio(&ctx);
    let replacement = Design {
        title: "Geladener Entwurf".into(),
        grid_y: 3,
        ..Design::default()
    };
    let path = fixture.root.join("valid.gridfinity.json");
    fs::write(&path, serde_json::to_vec(&replacement).unwrap()).unwrap();
    app.load_path(path.clone());
    assert_eq!(app.key(), serde_json::to_string(&replacement).unwrap());
    assert_eq!(app.saved_path, Some(path));
    assert_eq!(app.key(), app.saved_key);
    assert!(app.pending_transition.is_none());
    assert!(app.operation_error.is_none());
    assert!(!app.fresh());
}

#[test]
fn add_actions_stop_at_the_element_limit_without_changing_the_design() {
    let fixture = Fixture::new();
    let ctx = egui::Context::default();
    let mut app = fixture.studio(&ctx);
    for index in 0..40 {
        app.add_wall(index % 2 == 0);
    }
    assert_eq!(app.cfg.walls.len(), 40);
    let before = app.key();
    let undo_count = app.wall_editor.undo.len();
    app.add_wall(true);
    app.add_wall(false);
    assert_eq!(app.key(), before);
    assert_eq!(app.wall_editor.undo.len(), undo_count);
    assert!(app.operation_error.as_deref().unwrap().contains("40"));

    app.cfg = Design {
        product: Product::Insert,
        ..Design::default()
    };
    app.operation_error = None;
    for _ in 0..40 {
        app.add_cutout();
    }
    assert_eq!(app.cfg.cutouts.len(), 40);
    let before = app.key();
    app.add_cutout();
    assert_eq!(app.key(), before);
    assert!(app.operation_error.as_deref().unwrap().contains("40"));
}
