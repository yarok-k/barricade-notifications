use gtk::prelude::*;
use crate::{lib::widget::WidgetTrait, widgets::calendar};
use anyhow::{Result,Error};
pub struct Calendar{
    pub container: gtk::Box,
    //calendar: gtk::Calendar,
}
impl WidgetTrait for Calendar{
    fn new() -> Result<Self> {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.style_context().add_class("barricade_applet_widget_background");
        let calendar = gtk::Calendar::new();
        calendar.style_context().add_class("barricade_calendar");
        container.pack_start(&calendar, false, false, 0);
        container.show_all();
        Ok(Self { container})//, calendar })
    }
    fn update(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
