use std::collections::BinaryHeap;
use std::cmp::{Ordering, Ord};

use core::application;
use ecs;
use graphics;

use math;
use math::{Matrix, SquareMatrix, MetricSpace, InnerSpace, Transform as MT};

use super::errors::*;
use super::{Transform, Decomposed, Mesh, Renderable, RenderCamera, RenderEnvironment, PointLight};

pub struct MeshRenderer {}

impl MeshRenderer {
    pub fn new(_: &mut application::Application) -> Result<Self> {
        Ok(MeshRenderer {})
    }

    pub fn draw(&mut self,
                mut application: &mut application::Application,
                world: &ecs::World,
                env: &RenderEnvironment,
                camera: &RenderCamera)
                -> Result<()> {

        let (view, mut arenas) = world.view_with_2::<Transform, Mesh>();

        for v in view {
            self.submit(&mut application, &env, &camera, &mut arenas, v)
                .ok();
        }

        Ok(())
    }

    fn submit(&self,
              application: &mut application::Application,
              env: &RenderEnvironment,
              camera: &RenderCamera,
              arenas: &mut (ecs::ArenaGetter<Transform>, ecs::ArenaGetter<Mesh>),
              v: ecs::Entity)
              -> Result<()> {
        use graphics::UniformVariable as UV;
        use graphics::UniformVariableType as UVT;

        let mesh = arenas.1.get(*v).unwrap();
        if !mesh.is_visible() || mesh.material().is_none() {
            return Ok(());
        }

        let position = Transform::world_position(&arenas.0, v)?;
        let csp = camera.into_view_space(&position);
        if !camera.is_inside(&csp) {
            return Ok(());
        }

        // Update material.
        let mat = mesh.material().unwrap();
        let mut mat = mat.write().unwrap();
        mat.update_video_object(&mut application.graphics)?;

        // Get pipeline state object from shader.
        let pso = {
            let mut shader = mat.shader().write().unwrap();
            shader.update_video_object(&mut application.graphics)?;
            shader.video_object().unwrap()
        };

        // Get primitive buffer objects from mesh.
        let (vbo, ibo, len) = {
            let mut primitive = mesh.primitive().write().unwrap();
            primitive.update_video_object(&mut application.graphics)?;

            let (vbo, ibo) = primitive.video_object().unwrap();
            let len = if ibo.is_none() {
                primitive.vlen()
            } else {
                primitive.ilen().unwrap()
            };

            (vbo, ibo, len as u32)
        };

        // Submit draw call with packed order.
        let order = {
            let shader = mat.shader().read().unwrap();
            DrawOrder {
                tranlucent: shader.render_state().color_blend.is_some(),
                zorder: (csp.z.min(camera.clip.0).max(camera.clip.1) * 1000f32) as u32,
                pso: pso,
            }
        };

        // Create drawcall task.
        let mut drawcall = application.graphics.create_frame_task();

        // Extract uniform variables specified in material.
        mat.extract(&mut drawcall);

        // Assemble uniform variables with build-in uniforms.
        // Transformations.
        let m = Transform::as_matrix(&arenas.0, v)?;
        if mat.has_uniform_variable("bi_ModelMatrix", UVT::Matrix4f) {
            drawcall.with_uniform_variable("bi_ModelMatrix", m.into());
        }

        let vm = camera.view * m;
        if mat.has_uniform_variable("bi_ViewModelMatrix", UVT::Matrix4f) {
            drawcall.with_uniform_variable("bi_ViewModelMatrix", vm.into());
        }

        if mat.has_uniform_variable("bi_NormalMatrix", UVT::Matrix4f) {
            // Use a special normal matrix to remove the effect of wrongly scaling the normal
            // vector with `bi_ViewModelMatrix`.
            let n = if let Some(normal) = vm.invert() {
                normal.transpose()
            } else {
                vm
            };

            drawcall.with_uniform_variable("bi_NormalMatrix", n.into());
        }

        // Lights.
        let mut nearest_dir = None;
        let min_dis = ::std::f32::MAX;
        for v in &env.directional_lights {
            let dis = v.0.disp.distance2(position);
            if dis < min_dis {
                nearest_dir = Some(v);
            }
        }

        if let Some(v) = nearest_dir {
            if mat.has_uniform_variable("bi_DirLightEyeDir", UVT::Vector3f) {
                let dir = v.0
                    .transform_vector(math::Vector3::unit_z() * -1.0)
                    .normalize();
                drawcall.with_uniform_variable("bi_DirLightEyeDir", dir.into());
            }

            if mat.has_uniform_variable("bi_DirLightColor", UVT::Vector3f) {
                drawcall.with_uniform_variable("bi_DirLightColor", UV::Vector3f(v.1.color.rgb()));
            }
        }

        let mut heap = BinaryHeap::new();
        for v in &env.point_lights {
            let dis = v.0.disp.distance2(position);
            heap.push(PointLightInstance(dis, v.0, v.1));
        }

        for i in 0..4 {
            if let Some(v) = heap.pop() {
                let field = format!("bi_PointLightEyePos[{:?}]", i);
                if mat.has_uniform_variable(&field, UVT::Vector3f) {
                    let pos = v.1.disp;
                    let elp = camera.view * math::Vector4::new(pos.x, pos.y, pos.z, 1.0);
                    let elp = math::Vector3::new(elp.x, elp.y, elp.z);
                    drawcall.with_uniform_variable(&field, elp.into());
                }

                let field = format!("bi_PointLightColor[{:?}]", i);
                if mat.has_uniform_variable(&field, UVT::Vector3f) {
                    drawcall.with_uniform_variable(&field, UV::Vector3f(v.2.color.rgb()));
                }

                let field = format!("bi_PointLightAttenuation[{:?}]", i);
                if mat.has_uniform_variable(&field, UVT::Vector3f) {
                    let attenuation = math::Vector3::new(1.0, 0.0, 0.0);
                    drawcall.with_uniform_variable(&field, attenuation.into());
                }
            }
        }

        // TODO: Optimize uniform variable that shared by all the objects into one request
        // per frame.
        if mat.has_uniform_variable("bi_ViewMatrix", UVT::Matrix4f) {
            drawcall.with_uniform_variable("bi_ViewMatrix", camera.view.into());
        }

        if mat.has_uniform_variable("bi_ProjectionMatrix", UVT::Matrix4f) {
            drawcall.with_uniform_variable("bi_ProjectionMatrix", camera.projection.into());
        }

        if mat.has_uniform_variable("bi_AmbientColor", UVT::Vector3f) {
            let v: [f32; 4] = env.ambient.into();
            let color = math::Vector4::from(v) * 0.5;
            drawcall.with_uniform_variable("bi_AmbientColor", color.truncate().into());
        }

        drawcall
            .with_order(order.into())
            .with_view(camera.vso)
            .with_pipeline(pso)
            .with_data(vbo, ibo)
            .submit(graphics::Primitive::Triangles, 0, len)?;
        Ok(())
    }
}

struct DrawOrder {
    pub tranlucent: bool,
    pub zorder: u32,
    pub pso: graphics::PipelineStateHandle,
}

impl Into<u64> for DrawOrder {
    fn into(self) -> u64 {
        let prefix = if self.tranlucent {
            (!self.zorder)
        } else {
            self.zorder
        };

        let suffix = self.pso.index();
        ((prefix as u64) << 32) | (suffix as u64)
    }
}

#[derive(Debug)]
struct PointLightInstance(f32, Decomposed, PointLight);

impl PartialEq for PointLightInstance {
    fn eq(&self, rhs: &Self) -> bool {
        self.0 == rhs.0
    }
}

impl Eq for PointLightInstance {}

impl Ord for PointLightInstance {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.partial_cmp(&rhs).unwrap()
    }
}

impl PartialOrd for PointLightInstance {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&rhs.0)
    }
}