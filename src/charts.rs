// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Static chart geometry shared by every view and export.
use crate::model::{Bounds, Kind};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Column,
    Bar,
    Line,
    Pie,
}
impl Family {
    pub fn for_kind(k: Kind) -> Option<Self> {
        match k {
            Kind::ColumnChart => Some(Self::Column),
            Kind::BarChart => Some(Self::Bar),
            Kind::LineChart => Some(Self::Line),
            Kind::PieChart => Some(Self::Pie),
            _ => None,
        }
    }
    pub fn for_id(id: &str) -> Option<Self> {
        match id {
            "column_chart" => Some(Self::Column),
            "bar_chart" => Some(Self::Bar),
            "line_chart" => Some(Self::Line),
            "pie_chart" => Some(Self::Pie),
            _ => None,
        }
    }
    pub fn default_size(self) -> (f32, f32) {
        if self == Self::Pie {
            (180., 180.)
        } else {
            (187., 175.)
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Bar {
    pub frame: Bounds,
    pub fill: Bounds,
    pub color: [u8; 3],
}
#[derive(Clone, Debug, Serialize)]
pub struct Stroke {
    pub points: Vec<[f32; 2]>,
    pub width: f32,
    pub color: [u8; 3],
}
#[derive(Clone, Debug, Serialize)]
pub struct Pie {
    pub background: Bounds,
    pub center: [f32; 2],
    pub radius: f32,
    pub width: f32,
    pub divider: Vec<[f32; 2]>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub bars: Vec<Bar>,
    pub series: Vec<Stroke>,
    pub axes: Vec<Stroke>,
    pub pie: Option<Pie>,
    pub ink: [u8; 3],
}
pub fn layout(f: Family, w: f32, h: f32, sketch: bool) -> Layout {
    let ink = if sketch { [0; 3] } else { [51; 3] };
    let axis = if sketch { 4. } else { 2. };
    let mut l = Layout {
        bars: vec![],
        series: vec![],
        axes: vec![],
        pie: None,
        ink,
    };
    match f {
        Family::Column | Family::Bar => {
            l.axes = vec![
                Stroke {
                    points: vec![[axis / 2., 0.], [axis / 2., h]],
                    width: axis,
                    color: ink,
                },
                Stroke {
                    points: vec![[0., h - axis / 2.], [w, h - axis / 2.]],
                    width: axis,
                    color: ink,
                },
            ];
            let mut emitted = 0;
            for (i, fraction) in [0.16, 0.31, 0.58, 0.73].into_iter().enumerate() {
                let b = if f == Family::Column {
                    let length = (h - [89., 67., 32., 48.][i]).max(10.);
                    Bounds::new(
                        (w * fraction).round(),
                        h - length - axis + 1.,
                        (w * 0.15).round(),
                        length,
                    )
                } else {
                    Bounds::new(
                        axis - 1.,
                        (h * fraction).round(),
                        (w - [100., 79., 45., 60.][i]).max(10.),
                        (h * 0.15).round(),
                    )
                };
                if b.w <= 0. || b.h <= 0. {
                    continue;
                }
                let fill = Bounds::new(b.x + 1., b.y + 1., (b.w - 2.).max(1.), (b.h - 2.).max(1.));
                l.bars.push(Bar {
                    frame: b,
                    fill,
                    color: if emitted % 2 == 0 { [223; 3] } else { [149; 3] },
                });
                emitted += 1;
            }
        }
        Family::Line => {
            let thickness = if sketch {
                (w.max(h) / 40.).clamp(4., 8.)
            } else {
                2.
            };
            l.series = vec![
                Stroke {
                    points: vec![
                        [5., h - 5.],
                        [w / 3., h / 3.],
                        [w * 2. / 3., h / 2.],
                        [w - 15., 25.],
                    ],
                    width: thickness,
                    color: [230; 3],
                },
                Stroke {
                    points: vec![
                        [5., h * 123. / 173.],
                        [w / 3., h * 3. / 5.],
                        [w * 2. / 3., h / 5.],
                        [w - 15., h / 3.],
                    ],
                    width: thickness,
                    color: [167; 3],
                },
            ];
            l.axes.push(Stroke {
                points: vec![
                    [axis / 2., axis / 2.],
                    [axis / 2., h - axis / 2.],
                    [w - axis / 2., h - axis / 2.],
                ],
                width: axis,
                color: ink,
            });
        }
        Family::Pie => {
            let d = w.min(h);
            let t = if sketch { (d / 40.).clamp(4., 8.) } else { 2. };
            let c = [w / 2., h / 2.];
            let radius = d / 2. - t / 2.;
            let angle = 120.0_f32.to_radians();
            l.pie = Some(Pie {
                background: Bounds::new(
                    (w - d) / 2. + t,
                    (h - d) / 2. + t,
                    (d - 2. * t).max(1.),
                    (d - 2. * t).max(1.),
                ),
                center: c,
                radius,
                width: t,
                divider: vec![
                    [c[0], (h - d) / 2. + t],
                    c,
                    [c[0] + radius * angle.cos(), c[1] + radius * angle.sin()],
                ],
            });
        }
    }
    l
}
