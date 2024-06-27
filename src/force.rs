use std::sync::Arc;

use egui::{Rect, Vec2};
use glow::{self, HasContext};

const VERTEX_SHADER_SRC: &str = include_str!("shaders/vertex.glsl");

const FRAGMENT_SHADER_SRC: &str = include_str!("shaders/repulsive_force.glsl");

unsafe fn compile_shader(gl: &glow::Context, source: &str, kind: u32) -> glow::Shader {
    let shader = gl.create_shader(kind).unwrap();
    gl.shader_source(shader, source);
    gl.compile_shader(shader);
    if !gl.get_shader_compile_status(shader) {
        panic!("{}", gl.get_shader_info_log(shader));
    }
    shader
}

unsafe fn link_program(gl: &glow::Context, vertex_shader: &str, fragment_shader: &str) -> glow::Program {
    let program = gl.create_program().unwrap();
    let vs = compile_shader(gl, vertex_shader, glow::VERTEX_SHADER);
    let fs = compile_shader(gl, fragment_shader, glow::FRAGMENT_SHADER);
    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!("{}", gl.get_program_info_log(program));
    }
    program
}

unsafe fn create_texture(gl: &glow::Context, width: i32, height: i32, data: Option<&[u8]>) -> glow::Texture {
    let texture = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA32F as i32,
        width,
        height,
        0,
        glow::RGBA,
        glow::FLOAT,
        data,
    );
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
    texture
}

unsafe fn create_framebuffer(gl: &glow::Context, texture: glow::Texture) -> glow::Framebuffer {
    let framebuffer = gl.create_framebuffer().unwrap();
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
    gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(texture), 0);
    framebuffer
}

pub struct ForceCalculator {
    gl: Arc<glow::Context>,
    program: glow::Program,
}

impl ForceCalculator {
    pub fn new(gl: Arc<glow::Context>) -> Self {
        let program = unsafe { link_program(&gl, VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC) };
        Self { gl, program }
    }

    pub fn calculate_forces(&self, scale: f32, repulsion_radius: f32, rects: &[Rect]) -> Vec<Vec2> {
        let num_rects = rects.len();

        let input: Vec<f32> = rects.iter().flat_map(|rect| {
            let center = rect.center();
            [center.x, center.y, rect.width(), rect.height()]
        }).collect();

        let mut result: Vec<Vec2> = vec![Vec2::ZERO; num_rects];

        unsafe {
            let rect_texture = create_texture(&self.gl, num_rects as i32, 1, Some(bytemuck::cast_slice(&input)));

            let force_texture = create_texture(&self.gl, num_rects as i32, 1, None);
            let force_framebuffer = create_framebuffer(&self.gl, force_texture);

            self.gl.use_program(Some(self.program));
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(force_framebuffer));
            self.gl.bind_texture(glow::TEXTURE_2D, Some(rect_texture));
            self.gl.uniform_1_f32(self.gl.get_uniform_location(self.program, "u_repulsionRadius").as_ref(), repulsion_radius);
            self.gl.uniform_1_f32(self.gl.get_uniform_location(self.program, "u_scaleSquared").as_ref(), scale * scale);
            self.gl.uniform_1_i32(self.gl.get_uniform_location(self.program, "u_numRects").as_ref(), num_rects as i32);
            self.gl.uniform_1_i32(self.gl.get_uniform_location(self.program, "u_rects").as_ref(), 0);

            let vertex_array = self.gl.create_vertex_array().unwrap();
            self.gl.bind_vertex_array(Some(vertex_array));

            // TODO: How do I run the program? -> Need to create a rect out of vertices and draw them...
            self.gl.viewport(0, 0, num_rects as i32, 1);
            self.gl.draw_arrays(glow::TRIANGLES, 0, 6);

            let mut data: Vec<f32> = vec![0.5; num_rects * 4];
            self.gl.read_pixels(
                0,
                0,
                num_rects as i32,
                1,
                glow::RGBA,
                glow::FLOAT,
                glow::PixelPackData::Slice(bytemuck::cast_slice_mut(&mut data))
            );

            // web_sys::console::log_1(&format!("data: {:?}", data).into());

            for i in 0..num_rects {
                let x = data[i * 4];
                let y = data[i * 4 + 1];
                result[i] = Vec2::new(x, y);
            }

            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }

        result
    }

}
