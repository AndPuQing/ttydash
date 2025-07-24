use color_eyre::Result;
use ratatui::{layout::{Rect, Size}, Frame};
use tokio::sync::mpsc;

use crate::{
    action::Action,
    components::{dash::Dash, Component},
    config::Config,
    tui::Event,
};

pub struct ComponentManager {
    components: Vec<Box<dyn Component>>,
}

impl ComponentManager {
    pub fn new(args: crate::cli::Cli) -> Self {
        Self {
            components: vec![Box::new(Dash::new(args))],
        }
    }

    pub fn register_action_handler(&mut self, tx: mpsc::UnboundedSender<Action>) -> Result<()> {
        for component in self.components.iter_mut() {
            component.register_action_handler(tx.clone())?;
        }
        Ok(())
    }

    pub fn register_config_handler(&mut self, config: Config) -> Result<()> {
        for component in self.components.iter_mut() {
            component.register_config_handler(config.clone())?;
        }
        Ok(())
    }

    pub fn init(&mut self, area: Size) -> Result<()> {
        for component in self.components.iter_mut() {
            component.init(area)?;
        }
        Ok(())
    }

    pub fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>> {
        for component in self.components.iter_mut() {
            if let Some(action) = component.handle_events(event.clone())? {
                return Ok(Some(action));
            }
        }
        Ok(None)
    }

    pub fn update(&mut self, action: Action) -> Result<Option<Action>> {
        for component in self.components.iter_mut() {
            if let Some(action) = component.update(action.clone())? {
                return Ok(Some(action));
            }
        }
        Ok(None)
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        for component in self.components.iter_mut() {
            component.draw(frame, area)?;
        }
        Ok(())
    }
}