use std::{collections::HashMap, sync::Arc};

use ratatui::{
    prelude::{Backend, Rect},
    style::{Color, Stylize},
    symbols::Marker,
    widgets::{
        canvas::{Canvas, Painter, Shape, MapResolution, Map},
        Block,
    },
    Frame,
};

use crate::ui::{Draw, data::LatLon};

pub(crate) struct MapInfo {
    width: u16,
    ip_location_map: Arc<HashMap<String, Option<LatLon>>>
}

impl Draw for MapInfo {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        self.width = f.size().width / 2;

        let canvas = Canvas::default()
            .block(Block::default())
            .marker(Marker::Braille)
            .paint(|ctx| {
                ctx.draw(&Map {
                    color: Color::Green,
                    resolution: MapResolution::High,
                });

                for (host, latlon) in self.ip_location_map.iter() {
                    if let Some(loc) = latlon {
                        ctx.print(loc.lon.into(), loc.lat.into(), "X".yellow());
                    }
                }
            })
            .x_bounds([-180.0, 180.0])
            .y_bounds([-90.0, 90.0]);

        f.render_widget(canvas, area);
    }
}

impl Shape for MapInfo {
    fn draw(&self, painter: &mut Painter) {
        for i in 0..200 {
            let color = Color::LightBlue;
            let (x, y) = (i % (self.width as usize), i / (self.width as usize));
            painter.paint(x, y, color);
        }
    }
}

impl MapInfo {
    pub(crate) fn new(
        ip_location_map: Arc<HashMap<String, Option<LatLon>>>
    ) -> Self {
        MapInfo {
            width: 20,
            ip_location_map
        }
    }
}
