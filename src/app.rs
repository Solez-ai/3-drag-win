use crate::autostart;
use crate::commands::AppCommand;
use crate::config::{AppConfig, AppPaths, GestureAction};
use crate::ffi::{self, Point};
use crate::logging;
use crate::settings_ui::{self, SettingsWindowHandle};
use crate::single_instance;
use crate::touchpad::{self, TouchpadEvent};
use crate::tray;
use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, select, unbounded};
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
enum InputEvent {
    GestureStart,
    GestureDelta { dx: f64, dy: f64 },
    GestureEnd,
}

#[derive(Debug)]
struct DragSession {
    action: GestureAction,
    handle: Option<i64>,
    anchor_window: Option<Point>,
    anchor_cursor: Point,
    total_delta_x: f64,
    total_delta_y: f64,
    last_applied: Point,
    last_tick: Instant,
}

#[derive(Debug)]
struct DragController {
    config: AppConfig,
    session: Option<DragSession>,
}

struct TouchpadRuntime {
    _handle: touchpad::ListenerHandle,
    receiver: Receiver<TouchpadEvent>,
}

pub fn run() -> Result<()> {
    let _single_instance = single_instance::acquire()?;
    let paths = AppPaths::resolve()?;
    logging::init(&paths)?;
    ffi::hide_console_window();
    ffi::bootstrap_process();

    let mut config = match AppConfig::load_or_create(&paths) {
        Ok(config) => config,
        Err(error) => {
            log::error!("{error:#}");
            AppConfig::default()
        }
    };

    autostart::synchronize(config.launch_at_startup)?;
    persist_config(&paths, &config);

    log::info!(
        "starting 3-win-drag | profile={} | action={} | fingers={} | sensitivity={} | auto_start={} | config={}",
        config.touchpad_profile,
        config.gesture_action.label(),
        config.gesture_finger_count,
        config.touchpad_sensitivity,
        config.launch_at_startup,
        paths.config_path().display()
    );

    let (command_tx, command_rx) = unbounded();
    let _tray = tray::build(command_tx.clone(), &paths, &config)?;
    let settings_window = settings_ui::spawn_window(paths.clone(), command_tx.clone())?;

    let mut touchpad_runtime = spawn_touchpad_runtime(&config)?;
    let mut controller = DragController::new(config.clone());

    event_loop(
        &paths,
        &mut config,
        &mut controller,
        &mut touchpad_runtime,
        command_rx,
        &settings_window,
    )
}

fn spawn_touchpad_runtime(config: &AppConfig) -> Result<TouchpadRuntime> {
    let (touchpad_tx, touchpad_rx) = unbounded();
    let handle = touchpad::spawn_listener(
        touchpad_tx,
        config.gesture_finger_count,
        config.touchpad_sensitivity,
    )?;

    Ok(TouchpadRuntime {
        _handle: handle,
        receiver: touchpad_rx,
    })
}

fn event_loop(
    paths: &AppPaths,
    config: &mut AppConfig,
    controller: &mut DragController,
    touchpad_runtime: &mut TouchpadRuntime,
    command_rx: Receiver<AppCommand>,
    settings_window: &SettingsWindowHandle,
) -> Result<()> {
    loop {
        select! {
            recv(touchpad_runtime.receiver) -> message => {
                match message {
                    Ok(event) => controller.handle_input(map_touchpad_event(event)),
                    Err(_) => return Ok(()),
                }
            }
            recv(command_rx) -> message => {
                match message {
                    Ok(command) => {
                        if handle_command(
                            paths,
                            config,
                            controller,
                            touchpad_runtime,
                            command,
                            settings_window,
                        )? {
                            break;
                        }
                    }
                    Err(_) => return Ok(()),
                }
            }
        }
    }

    Ok(())
}

fn map_touchpad_event(event: TouchpadEvent) -> InputEvent {
    match event {
        TouchpadEvent::GestureStart => InputEvent::GestureStart,
        TouchpadEvent::GestureDelta { dx, dy } => InputEvent::GestureDelta { dx, dy },
        TouchpadEvent::GestureEnd => InputEvent::GestureEnd,
    }
}

fn handle_command(
    paths: &AppPaths,
    config: &mut AppConfig,
    controller: &mut DragController,
    touchpad_runtime: &mut TouchpadRuntime,
    command: AppCommand,
    settings_window: &SettingsWindowHandle,
) -> Result<bool> {
    match command {
        AppCommand::EnableDragging => {
            config.enabled = true;
            controller.update_config(config.clone());
            persist_config(paths, config);
            settings_window.refresh();
            log::info!("dragging enabled");
        }
        AppCommand::DisableDragging => {
            config.enabled = false;
            controller.update_config(config.clone());
            controller.cancel_drag();
            persist_config(paths, config);
            settings_window.refresh();
            log::info!("dragging disabled");
        }
        AppCommand::EnableAutoStart => {
            config.launch_at_startup = true;
            autostart::set_enabled(true)?;
            persist_config(paths, config);
            settings_window.refresh();
            log::info!("auto-start enabled");
        }
        AppCommand::DisableAutoStart => {
            config.launch_at_startup = false;
            autostart::set_enabled(false)?;
            persist_config(paths, config);
            settings_window.refresh();
            log::info!("auto-start disabled");
        }
        AppCommand::OpenDataDirectory => {
            open_path(paths.data_dir())?;
        }
        AppCommand::OpenLogDirectory => {
            open_path(paths.log_dir())?;
        }
        AppCommand::OpenSettings => {
            settings_window.open()?;
        }
        AppCommand::ApplyConfig(new_config) => {
            match apply_config(paths, config, controller, touchpad_runtime, new_config) {
                Ok(()) => settings_window.refresh(),
                Err(error) => {
                    log::error!("failed to apply config: {error:#}");
                    ffi::show_error_dialog("3-win-drag settings error", &error.to_string());
                    settings_window.refresh();
                }
            }
        }
        AppCommand::Exit => {
            log::info!("exit requested");
            return Ok(true);
        }
    }

    Ok(false)
}

fn apply_config(
    paths: &AppPaths,
    config: &mut AppConfig,
    controller: &mut DragController,
    touchpad_runtime: &mut TouchpadRuntime,
    new_config: AppConfig,
) -> Result<()> {
    let requires_touchpad_restart = config.gesture_finger_count != new_config.gesture_finger_count
        || (config.touchpad_sensitivity - new_config.touchpad_sensitivity).abs() > f32::EPSILON;
    let auto_start_changed = config.launch_at_startup != new_config.launch_at_startup;

    *config = new_config;

    if auto_start_changed {
        autostart::set_enabled(config.launch_at_startup)?;
    }

    if requires_touchpad_restart {
        controller.cancel_drag();
        *touchpad_runtime = spawn_touchpad_runtime(config)?;
    }

    controller.update_config(config.clone());
    persist_config(paths, config);

    log::info!(
        "config applied | profile={} | action={} | fingers={} | sensitivity={} | deadzone={} | smoothing={}",
        config.touchpad_profile,
        config.gesture_action.label(),
        config.gesture_finger_count,
        config.touchpad_sensitivity,
        config.deadzone_pixels,
        config.smoothing_factor
    );

    Ok(())
}

fn open_path(path: &std::path::Path) -> Result<()> {
    Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .with_context(|| format!("failed to open {}", path.display()))?;
    Ok(())
}

fn persist_config(paths: &AppPaths, config: &AppConfig) {
    if let Err(error) = config.save(paths) {
        log::error!("{error:#}");
    }
}

impl DragController {
    fn new(config: AppConfig) -> Self {
        Self {
            config,
            session: None,
        }
    }

    fn update_config(&mut self, config: AppConfig) {
        if self.config.gesture_action == GestureAction::MouseDrag
            && config.gesture_action != GestureAction::MouseDrag
            && self.session.is_some()
        {
            let _ = ffi::mouse_left_button_up();
        }
        self.config = config;
        if !self.config.enabled {
            self.cancel_drag();
        }
    }

    fn cancel_drag(&mut self) {
        if let Some(session) = &self.session
            && session.action == GestureAction::MouseDrag
        {
            let _ = ffi::mouse_left_button_up();
        }
        self.session = None;
    }

    fn handle_input(&mut self, event: InputEvent) {
        match event {
            InputEvent::GestureStart => self.handle_gesture_start(),
            InputEvent::GestureDelta { dx, dy } => self.handle_gesture_delta(dx, dy),
            InputEvent::GestureEnd => self.handle_gesture_end(),
        }
    }

    fn handle_gesture_start(&mut self) {
        if !self.config.enabled || self.session.is_some() {
            return;
        }

        self.start_session();
    }

    fn handle_gesture_delta(&mut self, dx: f64, dy: f64) {
        if !self.config.enabled {
            return;
        }

        let Some(session) = self.session.as_mut() else {
            return;
        };

        session.total_delta_x += dx;
        session.total_delta_y += dy;
        let desired = match session.action {
            GestureAction::WindowMove => {
                let Some(handle) = session.handle else {
                    self.session = None;
                    return;
                };
                let Some(anchor_window) = session.anchor_window else {
                    self.session = None;
                    return;
                };

                if !ffi::window_is_valid(handle) {
                    self.session = None;
                    return;
                }

                let Some(cursor) = ffi::current_cursor_position() else {
                    return;
                };
                Point::new(
                    anchor_window.x + (cursor.x - session.anchor_cursor.x),
                    anchor_window.y + (cursor.y - session.anchor_cursor.y),
                )
            }
            GestureAction::MouseDrag => Point::new(
                session.anchor_cursor.x + session.total_delta_x.round() as i32,
                session.anchor_cursor.y + session.total_delta_y.round() as i32,
            ),
        };

        let reference = match session.action {
            GestureAction::WindowMove => session.anchor_window.unwrap_or(session.last_applied),
            GestureAction::MouseDrag => session.anchor_cursor,
        };
        let total_dx = desired.x - reference.x;
        let total_dy = desired.y - reference.y;
        if total_dx.abs() <= self.config.deadzone_pixels
            && total_dy.abs() <= self.config.deadzone_pixels
        {
            return;
        }

        let next = smooth_point(session.last_applied, desired, self.config.smoothing_factor);
        if next == session.last_applied {
            return;
        }

        let interval = Duration::from_millis(self.config.minimum_update_interval_ms.max(1));
        if session.last_tick.elapsed() < interval
            && (next.x - session.last_applied.x).abs() <= 1
            && (next.y - session.last_applied.y).abs() <= 1
        {
            return;
        }

        match session.action {
            GestureAction::WindowMove => {
                if let Some(handle) = session.handle {
                    if ffi::move_window(handle, next) {
                        session.last_applied = next;
                        session.last_tick = Instant::now();
                    } else {
                        self.session = None;
                    }
                } else {
                    self.session = None;
                }
            }
            GestureAction::MouseDrag => {
                if ffi::set_cursor_position(next) {
                    session.last_applied = next;
                    session.last_tick = Instant::now();
                }
            }
        }
    }

    fn handle_gesture_end(&mut self) {
        if let Some(session) = &self.session
            && session.action == GestureAction::MouseDrag
        {
            let _ = ffi::mouse_left_button_up();
        }
        self.session = None;
    }

    fn start_session(&mut self) {
        let cursor = ffi::current_cursor_position().unwrap_or(Point::new(0, 0));
        match self.config.gesture_action {
            GestureAction::WindowMove => {
                let Some(window) = ffi::prepare_foreground_window(cursor) else {
                    return;
                };

                log::info!(
                    "three-finger window drag started | hwnd={} | x={} | y={} | w={} | h={} | restored={}",
                    window.handle,
                    window.position.x,
                    window.position.y,
                    window.width,
                    window.height,
                    window.was_maximized
                );

                self.session = Some(DragSession {
                    action: GestureAction::WindowMove,
                    handle: Some(window.handle),
                    anchor_window: Some(window.position),
                    anchor_cursor: cursor,
                    total_delta_x: 0.0,
                    total_delta_y: 0.0,
                    last_applied: window.position,
                    last_tick: Instant::now(),
                });
            }
            GestureAction::MouseDrag => {
                if !ffi::mouse_left_button_down() {
                    return;
                }

                log::info!(
                    "three-finger mouse drag started | cursor_x={} | cursor_y={}",
                    cursor.x,
                    cursor.y
                );

                self.session = Some(DragSession {
                    action: GestureAction::MouseDrag,
                    handle: None,
                    anchor_window: None,
                    anchor_cursor: cursor,
                    total_delta_x: 0.0,
                    total_delta_y: 0.0,
                    last_applied: cursor,
                    last_tick: Instant::now(),
                });
            }
        }
    }
}

fn smooth_point(current: Point, target: Point, factor: f32) -> Point {
    let alpha = factor.clamp(0.1, 1.0);
    if (alpha - 1.0).abs() < f32::EPSILON {
        return target;
    }

    let dx = target.x - current.x;
    let dy = target.y - current.y;
    Point::new(
        current.x + scaled_step(dx, alpha),
        current.y + scaled_step(dy, alpha),
    )
}

fn scaled_step(delta: i32, alpha: f32) -> i32 {
    if delta == 0 {
        return 0;
    }

    let step = (delta as f32 * alpha).round() as i32;
    if step == 0 { delta.signum() } else { step }
}

#[cfg(test)]
mod tests {
    use super::{Point, scaled_step, smooth_point};

    #[test]
    fn smoothing_factor_one_moves_directly_to_target() {
        let current = Point::new(10, 20);
        let target = Point::new(40, 80);
        assert_eq!(smooth_point(current, target, 1.0), target);
    }

    #[test]
    fn smoothing_factor_applies_a_non_zero_step() {
        let current = Point::new(0, 0);
        let target = Point::new(10, 10);
        assert_eq!(smooth_point(current, target, 0.5), Point::new(5, 5));
        assert_eq!(scaled_step(1, 0.1), 1);
        assert_eq!(scaled_step(-1, 0.1), -1);
    }
}
