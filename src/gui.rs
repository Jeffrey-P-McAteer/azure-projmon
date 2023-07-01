
use three_d::*;


pub async fn render_ui_on_projector(
  headless_display_name: &str,
  connected_projector_name: &str,
  connected_projector_ws: &str
) {
  let window = Window::new(WindowSettings {
      title: "Logo!".to_string(),
      max_size: Some((512, 512)),
      ..Default::default()
  })
  .unwrap();
  let context = window.gl();

  let mut camera = Camera::new_perspective(
      window.viewport(),
      vec3(0.0, 0.0, 2.2),
      vec3(0.0, 0.0, 0.0),
      vec3(0.0, 1.0, 0.0),
      degrees(60.0),
      0.1,
      10.0,
  );

  let mut loaded = three_d_asset::io::load_async(&[
          "https://asny.github.io/three-d/assets/rust_logo.png",
      ])
      .await
      .expect("failed to download the necessary assets");

  let image = Texture2D::new(&context, &loaded.deserialize("").unwrap());

  let positions = vec![
      vec3(0.55, -0.4, 0.0),  // bottom right
      vec3(-0.55, -0.4, 0.0), // bottom left
      vec3(0.0, 0.6, 0.0),    // top
  ];
  let colors = vec![
      Color::new(255, 0, 0, 255), // bottom right
      Color::new(0, 255, 0, 255), // bottom left
      Color::new(0, 0, 255, 255), // top
  ];
  let cpu_mesh = CpuMesh {
      positions: Positions::F32(positions),
      colors: Some(colors),
      ..Default::default()
  };

  // Construct a model, with a default color material, thereby transferring the mesh data to the GPU
  let model = Gm::new(Mesh::new(&context, &cpu_mesh), ColorMaterial::default());

  window.render_loop(move |frame_input| {
      camera.set_viewport(frame_input.viewport);

      let geometries: Vec<Axes> = vec![];

      frame_input
          .screen()
          .clear(ClearState::color_and_depth(1.0, 1.0, 1.0, 1.0, 1.0))
          //.apply_screen_material(&LogoMaterial { image: &image }, &camera, &[])
          //.render(&camera, &model, &[]);
          .render_with_material(&LogoMaterial { image: &image }, &camera, &geometries, &[]);

      FrameOutput::default()
  });

}

struct LogoMaterial<'a> {
    image: &'a Texture2D,
}

impl Material for LogoMaterial<'_> {
    /*fn fragment_shader_source(&self, _lights: &[&dyn Light]) -> String {
        //include_str!("shader.frag").to_string()
        r#"
uniform sampler2D image;

in vec2 uvs;

layout (location = 0) out vec4 outColor;

vec3 srgb_from_rgb(vec3 rgb) {
    vec3 a = vec3(0.055, 0.055, 0.055);
    vec3 ap1 = vec3(1.0, 1.0, 1.0) + a;
    vec3 g = vec3(2.4, 2.4, 2.4);
    vec3 ginv = 1.0 / g;
    vec3 select = step(vec3(0.0031308, 0.0031308, 0.0031308), rgb);
    vec3 lo = rgb * 12.92;
    vec3 hi = ap1 * pow(rgb, ginv) - a;
    return mix(lo, hi, select);
}

void main()
{
    outColor = vec4(max(uvs.x, 1.0 - uvs.x), uvs.y, 1.0 - uvs.y, texture(image, uvs).g);
}
"#.to_string()
    }
    */
    fn fragment_shader(&self, _lights: &[&dyn Light]) -> FragmentShader {
      FragmentShader {
        source: r#"
uniform sampler2D image;

in vec2 uvs;

layout (location = 0) out vec4 outColor;

vec3 srgb_from_rgb(vec3 rgb) {
    vec3 a = vec3(0.055, 0.055, 0.055);
    vec3 ap1 = vec3(1.0, 1.0, 1.0) + a;
    vec3 g = vec3(2.4, 2.4, 2.4);
    vec3 ginv = 1.0 / g;
    vec3 select = step(vec3(0.0031308, 0.0031308, 0.0031308), rgb);
    vec3 lo = rgb * 12.92;
    vec3 hi = ap1 * pow(rgb, ginv) - a;
    return mix(lo, hi, select);
}

void main()
{
    outColor = vec4(max(uvs.x, 1.0 - uvs.x), uvs.y, 1.0 - uvs.y, texture(image, uvs).g);
}
"#.to_string(),
        attributes: FragmentAttributes {
          uv: true,
          ..FragmentAttributes::NONE
        }
      }
    }

    // fn id(&self) -> u16 {
    //     0b1u16
    // }

    // fn fragment_attributes(&self) -> FragmentAttributes {
    //     FragmentAttributes {
    //         uv: true,
    //         ..FragmentAttributes::NONE
    //     }
    // }

    fn use_uniforms(&self, program: &Program, _camera: &Camera, _lights: &[&dyn Light]) {
        program.use_texture("image", &self.image);
    }

    fn render_states(&self) -> RenderStates {
        RenderStates {
            write_mask: WriteMask::COLOR,
            blend: Blend::TRANSPARENCY,
            ..Default::default()
        }
    }

    fn material_type(&self) -> MaterialType {
        MaterialType::Transparent
    }
}



