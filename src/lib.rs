use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[wasm_bindgen]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Dead = 0,
    Alive = 1,
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct Universe {
    width: u32,
    height: u32,
    cells: Vec<Cell>,
}
#[wasm_bindgen]
impl Universe {
    fn get_index(&self, row: u32, column: u32) -> usize {
        (row * self.width + column) as usize
    }

    fn live_neighbor_count(&self, row: u32, column: u32) -> u8 {
        let mut count = 0;
        for delta_row in [self.height - 1, 0, 1].iter().cloned() {
            for delta_col in [self.width - 1, 0, 1].iter().cloned() {
                if delta_row == 0 && delta_col == 0 {
                    continue;
                }

                let neighbor_row = (row + delta_row) % self.height;
                let neighbor_col = (column + delta_col) % self.width;
                let idx = self.get_index(neighbor_row, neighbor_col);
                count += self.cells[idx] as u8;
            }
        }
        count
    }
    pub fn tick(&mut self) {
        let mut next = self.cells.clone();

        for row in 0..self.height {
            for col in 0..self.width {
                let idx = self.get_index(row, col);
                let cell = self.cells[idx];
                let live_neighbors = self.live_neighbor_count(row, col);

                let next_cell = match (cell, live_neighbors) {
                    // Rule 1: Any live cell with fewer than two live neighbours
                    // dies, as if caused by underpopulation.
                    (Cell::Alive, x) if x < 2 => Cell::Dead,
                    // Rule 2: Any live cell with two or three live neighbours
                    // lives on to the next generation.
                    (Cell::Alive, 2) | (Cell::Alive, 3) => Cell::Alive,
                    // Rule 3: Any live cell with more than three live
                    // neighbours dies, as if by overpopulation.
                    (Cell::Alive, x) if x > 3 => Cell::Dead,
                    // Rule 4: Any dead cell with exactly three live neighbours
                    // becomes a live cell, as if by reproduction.
                    (Cell::Dead, 3) => Cell::Alive,
                    // All other cells remain in the same state.
                    (otherwise, _) => otherwise,
                };

                next[idx] = next_cell;
            }
        }

        self.cells = next;
    }

    pub fn new() -> Universe {
        let width = 64;
        let height = 64;

        let cells = (0..width * height)
            .map(|i| {
                if i % 2 == 0 || i % 7 == 0 {
                    Cell::Alive
                } else {
                    Cell::Dead
                }
            })
            .collect();

        Universe {
            width,
            height,
            cells,
        }
    }

    pub fn render(&self) -> String {
        self.to_string()
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn cells(&self) -> *const Cell {
        self.cells.as_ptr()
    }
}

use std::fmt;

impl fmt::Display for Universe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for line in self.cells.as_slice().chunks(self.width as usize) {
            for &cell in line {
                let symbol = if cell == Cell::Dead { '◻' } else { '◼' };
                write!(f, "{}", symbol)?;
            }
            write!(f, "\n")?;
        }

        Ok(())
    }
}

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let info = document.get_element_by_id("info").unwrap();
    info.set_inner_html("message");
    Ok(())
}

use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader};

#[wasm_bindgen(start)]
fn render_gl() -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("gl").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    let dim = 64 * (5 + 1) + 1;

    canvas.set_height(dim);
    canvas.set_width(dim);

    let context = canvas
        .get_context("webgl2")?
        .unwrap()
        .dyn_into::<WebGl2RenderingContext>()?;
    let vert_shader = compile_shader(
        &context,
        WebGl2RenderingContext::VERTEX_SHADER,
        r##"#version 300 es
 
        in vec2 position;
        in float color;
        uniform mat4 model;

        out float outColor;

        void main() {
            gl_Position = model * vec4(position, 0, 1);
            outColor = color;
        }
        "##,
    )?;

    let frag_shader = compile_shader(
        &context,
        WebGl2RenderingContext::FRAGMENT_SHADER,
        r##"#version 300 es
    
        precision highp float;
        in float outColor;
        out vec4 diffuseColor;
        
        void main() {
            diffuseColor = vec4(outColor, outColor, outColor, 1);
        }
        "##,
    )?;
    let program = link_program(&context, &vert_shader, &frag_shader)?;
    context.use_program(Some(&program));

    let position_attribute_location = context.get_attrib_location(&program, "position");
    let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
    context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

    let vao = context
        .create_vertex_array()
        .ok_or("Could not create vertex array object")?;
    context.bind_vertex_array(Some(&vao));
    context.vertex_attrib_pointer_with_i32(
        position_attribute_location as u32,
        2,
        WebGl2RenderingContext::FLOAT,
        false,
        12,
        0,
    );
    context.enable_vertex_attrib_array(position_attribute_location as u32);

    let color_attribute_location = context.get_attrib_location(&program, "color");
    context.vertex_attrib_pointer_with_i32(
        color_attribute_location as u32,
        1,
        WebGl2RenderingContext::FLOAT,
        false,
        12,
        8,
    );
    context.enable_vertex_attrib_array(color_attribute_location as u32);
    let model_attribute_location = context.get_uniform_location(&program, "model");
    let left = 0f32;
    let right = dim as f32;
    web_sys::console::log_1(&right.to_string().into());
    let bottom = 0f32;
    let top = dim as f32;
    web_sys::console::log_1(&((right + left) / (right - left)).to_string().into());
    let far = -1f32;
    let near = 2f32;
    context.uniform_matrix4fv_with_f32_array(
        model_attribute_location.as_ref(),
        true,
        &[
            2f32 / (right - left),
            0f32,
            0f32,
            -(right + left) / (right - left),
            //row
            0f32,
            2f32 / (top - bottom),
            0f32,
            -(top + bottom) / (top - bottom),
            //row
            0f32,
            0f32,
            -2f32 / (far - near),
            -(far + near) / (far - near),
            // row
            0f32,
            0f32,
            0f32,
            1f32,
        ],
    );
    Ok(())
}

#[wasm_bindgen]
pub fn draw_universe(universe: &Universe) -> Result<(), JsValue> {
    // web_sys::console::log_1(&format!("{:?}", universe).into());
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("gl").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    let context = canvas
        .get_context("webgl2")?
        .unwrap()
        .dyn_into::<WebGl2RenderingContext>()?;

    draw(universe, &context);

    Ok(())
}

fn draw(universe: &Universe, context: &WebGl2RenderingContext) {
    context.clear_color(1.0, 1.0, 1.0, 1.0);

    context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
    draw_grid(universe, context);
    let size = 5f32;

    let gray = 0.1f32;
    for row in 0..universe.height() {
        for col in 0..universe.width() {
            let idx = universe.get_index(row, col);
            let cell = universe.cells[idx];
            let offset_x = (size + 1f32) * col as f32 + 1f32;
            let offset_y = (size + 1f32) * row as f32 + 1f32;
            if cell == Cell::Alive {
                let vertices = [
                    offset_x,
                    offset_y,
                    gray,
                    offset_x,
                    offset_y + size,
                    gray,
                    offset_x + size,
                    offset_y + size,
                    gray,
                    offset_x + size,
                    offset_y,
                    gray,
                ];
                draw_square(context, &vertices);
            }
        }
    }
}

fn draw_grid(universe: &Universe, context: &WebGl2RenderingContext) {
    // let size = 2f32 / universe.height() as f32;
    // let size = universe.height() as f32 / 4f32;
    let size = 5f32;
    let grid_length = 385f32;
    let gray = 0.6f32;
    for row in 0..=universe.height() {
        let offset = (size + 1f32) * row as f32;
        let vertices = [
            offset,
            0f32,
            gray,
            offset,
            grid_length,
            gray,
            0f32,
            offset,
            gray,
            grid_length,
            offset,
            gray,
        ];
        draw_line(context, &vertices);
    }
}

fn draw_line(context: &WebGl2RenderingContext, vertices: &[f32]) {
    unsafe {
        let positions_array_buf_view = js_sys::Float32Array::view(vertices);
        context.buffer_data_with_array_buffer_view(
            WebGl2RenderingContext::ARRAY_BUFFER,
            &positions_array_buf_view,
            WebGl2RenderingContext::STATIC_DRAW,
        );
    }
    let vert_count = (vertices.len() / 3) as i32;
    context.draw_arrays(WebGl2RenderingContext::LINES, 0, vert_count);
}

fn draw_square(context: &WebGl2RenderingContext, vertices: &[f32]) {
    unsafe {
        let positions_array_buf_view = js_sys::Float32Array::view(vertices);
        context.buffer_data_with_array_buffer_view(
            WebGl2RenderingContext::ARRAY_BUFFER,
            &positions_array_buf_view,
            WebGl2RenderingContext::STATIC_DRAW,
        );
    }
    let vert_count = (vertices.len() / 3) as i32;
    context.draw_arrays(WebGl2RenderingContext::TRIANGLE_FAN, 0, vert_count);
}

pub fn compile_shader(
    context: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGl2RenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
