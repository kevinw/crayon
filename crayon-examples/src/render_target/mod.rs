use crayon::prelude::*;

impl_vertex!{
    Vertex {
        position => [Position; Float; 2; false],
    }
}

struct Pass {
    view: graphics::ViewStateHandle,
    pipeline: graphics::PipelineStateHandle,
    mesh: graphics::VertexBufferHandle,
}

struct Window {
    _label: graphics::utils::RAIIGuard,
    pass: Pass,
    post_effect: Pass,
    texture: graphics::TextureHandle,
    time: f32,
}

impl Window {
    pub fn new(engine: &mut Engine) -> errors::Result<Self> {
        let ctx = engine.context().read().unwrap();
        let video = ctx.shared::<GraphicsSystem>().clone();
        let mut label = graphics::utils::RAIIGuard::new(video);

        let vertices: [Vertex; 3] = [Vertex::new([0.0, 0.5]),
                                     Vertex::new([0.5, -0.5]),
                                     Vertex::new([-0.5, -0.5])];

        let quad_vertices: [Vertex; 6] = [Vertex::new([-1.0, -1.0]),
                                          Vertex::new([1.0, -1.0]),
                                          Vertex::new([-1.0, 1.0]),
                                          Vertex::new([-1.0, 1.0]),
                                          Vertex::new([1.0, -1.0]),
                                          Vertex::new([1.0, 1.0])];

        let attributes = graphics::AttributeLayoutBuilder::new()
            .with(graphics::VertexAttribute::Position, 2)
            .finish();

        //
        let (pass, rendered_texture) = {
            // Create vertex buffer object.
            let mut setup = graphics::VertexBufferSetup::default();
            setup.num = vertices.len();
            setup.layout = Vertex::layout();
            let vbo = label
                .create_vertex_buffer(setup, Some(Vertex::as_bytes(&vertices[..])))?;

            // Create render texture for post effect.
            let mut setup = graphics::RenderTextureSetup::default();
            setup.format = graphics::RenderTextureFormat::RGBA8;
            setup.dimensions = (568, 320);
            let rendered_texture = label.create_render_texture(setup)?;

            // Create custom frame buffer.
            let mut setup = graphics::FrameBufferSetup::default();
            setup.set_texture_attachment(rendered_texture, Some(0))?;
            let fbo = label.create_framebuffer(setup)?;

            // Create the view state for pass 1.
            let mut setup = graphics::ViewStateSetup::default();
            setup.framebuffer = Some(fbo);
            setup.clear_color = Some(Color::gray());
            let view = label.create_view(setup)?;

            // Create pipeline state.
            let vs = include_str!("../../resources/render_target_p1.vs").to_owned();
            let fs = include_str!("../../resources/render_target_p1.fs").to_owned();
            let mut setup = graphics::PipelineStateSetup::default();
            setup.layout = attributes;
            let pipeline = label.create_pipeline(setup, vs, fs)?;

            (Pass {
                 view: view,
                 pipeline: pipeline,
                 mesh: vbo,
             },
             rendered_texture)
        };

        let post_effect = {
            let mut setup = graphics::VertexBufferSetup::default();
            setup.num = quad_vertices.len();
            setup.layout = Vertex::layout();
            let vbo = label
                .create_vertex_buffer(setup, Some(Vertex::as_bytes(&quad_vertices[..])))?;

            let setup = graphics::ViewStateSetup::default();
            let view = label.create_view(setup)?;

            let mut setup = graphics::PipelineStateSetup::default();
            setup.layout = attributes;
            let vs = include_str!("../../resources/render_target_p2.vs").to_owned();
            let fs = include_str!("../../resources/render_target_p2.fs").to_owned();
            let pipeline = label.create_pipeline(setup, vs, fs)?;

            Pass {
                view: view,
                pipeline: pipeline,
                mesh: vbo,
            }
        };

        Ok(Window {
               _label: label,

               pass: pass,
               post_effect: post_effect,
               texture: rendered_texture,

               time: 0.0,
           })
    }
}

impl Application for Window {
    fn on_update(&mut self, ctx: &Context) -> errors::Result<()> {
        let video = ctx.shared::<GraphicsSystem>();

        {
            video
                .make()
                .with_order(0)
                .with_view(self.pass.view)
                .with_pipeline(self.pass.pipeline)
                .with_data(self.pass.mesh, None)
                .submit(graphics::Primitive::Triangles, 0, 3)?;
        }

        {
            video
                .make()
                .with_order(1)
                .with_view(self.post_effect.view)
                .with_pipeline(self.post_effect.pipeline)
                .with_data(self.post_effect.mesh, None)
                .with_uniform_variable("time", self.time.into())
                .with_texture("renderedTexture", self.texture)
                .submit(graphics::Primitive::Triangles, 0, 6)?;
        }

        self.time += 0.05;
        Ok(())
    }
}

pub fn main(_: &[String]) {
    let mut settings = Settings::default();
    settings.window.width = 568;
    settings.window.height = 320;

    let mut engine = Engine::new_with(settings).unwrap();
    let window = Window::new(&mut engine).unwrap();
    engine.run(window).unwrap();
}