use wgpu_glyph::ab_glyph::PxScale;

pub mod mainmenu;
pub mod pausemenu;
pub mod widgets;

#[derive(Debug, Clone)]
pub struct RectanglePrimitive {
    pub layout: quint::Layout,
    pub color: [f32; 4],
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct TextPrimitive {
    pub x: i32,
    pub y: i32,
    pub w: Option<i32>,
    pub h: Option<i32>,
    pub parts: Vec<TextPart>,
    pub z: f32,
    pub center_horizontally: bool,
    pub center_vertically: bool,
}

#[derive(Debug, Clone)]
pub struct TrianglesPrimitive {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct TextPart {
    pub text: String,
    pub font_size: PxScale,
    pub color: [f32; 4],
    pub font: Option<String>,
}

#[derive(Default, Debug)]
pub struct PrimitiveBuffer {
    pub rectangle: Vec<RectanglePrimitive>,
    pub text: Vec<TextPrimitive>,
    pub triangles: Vec<TrianglesPrimitive>,
}

impl PrimitiveBuffer {
    pub fn draw_rectangle(&mut self, color: [f32; 4], layout: quint::Layout, z: f32) {
        self.rectangle.push(RectanglePrimitive { color, layout, z });
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: [f32; 4], z: f32) {
        self.rectangle.push(RectanglePrimitive {
            color,
            layout: quint::Layout {
                x: x as f32,
                y: y as f32,
                width: w as f32,
                height: h as f32,
            },
            z,
        });
    }

    /*pub fn draw_text(
        &mut self,
        parts: Vec<TextPart>,
        layout: quint::Layout,
        z: f32,
        centered: bool,
    ) {
        self.text.push(TextPrimitive {
            layout,
            parts,
            z,
            centered,
        })
    }*/

    pub fn draw_text_simple(
        &mut self,
        x: i32,
        y: i32,
        h: i32,
        text: String,
        color: [f32; 4],
        z: f32,
    ) {
        self.text.push(TextPrimitive {
            x,
            y,
            w: None,
            h: Some(h),
            parts: vec![TextPart {
                text,
                font_size: PxScale::from(20.0),
                color,
                font: None,
            }],
            z,
            center_horizontally: false,
            center_vertically: true,
        });
    }

    pub fn draw_triangles(&mut self, vertices: Vec<[f32; 3]>, indices: Vec<u32>, color: [f32; 4]) {
        self.triangles.push(TrianglesPrimitive {
            vertices,
            indices,
            color,
        });
    }
}
