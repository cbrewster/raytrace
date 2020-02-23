use nalgebra::{Matrix4, Point3, Point4, Vector3};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const SHADOW_BIAS: f32 = 0.0002;

trait Trace {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;
}

#[derive(Debug)]
struct Hit {
    distance: f32,
    normal: Vector3<f32>,
}

#[derive(Debug)]
struct Ray {
    origin: Point3<f32>,
    direction: Vector3<f32>,
}

#[derive(Debug)]
struct Light {
    position: Point3<f32>,
    intensity: f32,
}

#[derive(Debug)]
struct Sphere {
    position: Point3<f32>,
    radius: f32,
}

#[derive(Debug)]
struct Camera {
    position: Point3<f32>,
    look_at: Point3<f32>,
}

#[derive(Debug)]
enum Shape {
    Sphere(Sphere),
}

#[derive(Debug)]
struct Material {
    color: Vector3<f32>,
}

#[derive(Debug)]
struct Object {
    shape: Shape,
    material: Material,
}

#[derive(Debug)]
struct Scene {
    objects: Vec<Object>,
    camera: Camera,
    lights: Vec<Light>,
}

impl Ray {
    fn new(origin: Point3<f32>, direction: Vector3<f32>) -> Ray {
        Ray { origin, direction }
    }

    fn point_at_distance(&self, distance: f32) -> Point3<f32> {
        self.origin + distance * self.direction
    }
}

impl Light {
    fn new(position: Point3<f32>, intensity: f32) -> Light {
        Light { position, intensity }
    }
}

impl Object {
    fn sphere(position: Point3<f32>, radius: f32, color: Vector3<f32>) -> Object {
        Object {
            shape: Shape::Sphere(Sphere::new(position, radius)),
            material: Material { color },
        }
    }
}

impl Trace for Object {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        match &self.shape {
            Shape::Sphere(sphere) => sphere.intersect(ray),
        }
    }
}

impl Sphere {
    fn new(position: Point3<f32>, radius: f32) -> Sphere {
        Sphere {
            position,
            radius,
        }
    }
}

impl Trace for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let oc = ray.origin - self.position;
        let a = ray.direction.dot(&ray.direction);
        let b = 2.0 * oc.dot(&ray.direction);
        let c = oc.dot(&oc) - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant <= 0.0 {
            return None;
        }

        let disc2 = discriminant.sqrt();
        let t0 = (-b - disc2) / (2.0 * a);
        let t1 = (-b + disc2) / (2.0 * a);
        if t0 < 0.0 && t1 < 0.0 {
            return None;
        }

        let distance = if t0 < 0.0 { t1 } else { t0 };

        let hit_position = ray.point_at_distance(distance);
        let normal = (hit_position - self.position).normalize();

        Some(Hit {
            distance,
            normal,
        })
    }
}

impl Camera {
    fn new(position: Point3<f32>, look_at: Point3<f32>) -> Camera {
        Camera { position, look_at }
    }
}

impl Scene {
    fn new(objects: Vec<Object>, camera: Camera, lights: Vec<Light>) -> Scene {
        Scene {
            objects,
            camera,
            lights,
        }
    }

    // Get a color for a ray
    fn trace(&self, ray: &Ray) -> Vector3<f32> {
        let (hit, object) = match self.intersect(ray) {
            None => return Vector3::new(0.0, 0.0, 0.0),
            Some(hit) => hit,
        };

        let hit_point = ray.point_at_distance(hit.distance) + hit.normal * SHADOW_BIAS;
        let mut color = Vector3::new(0.0, 0.0, 0.0);

        for light in &self.lights {
            let shadow_ray_direction = (light.position - hit_point).normalize();
            let shadow_ray = Ray::new(hit_point, shadow_ray_direction);

            if self.intersect(&shadow_ray).is_some() {
                // This light does not contribute to this object
                continue;
            }

            let shade = f32::max(0.0, hit.normal.dot(&shadow_ray_direction));

            color += shade * object.material.color * light.intensity;
        }

        color
    }

    fn intersect(&self, ray: &Ray) -> Option<(Hit, &Object)> {
        self.objects
            .iter()
            .filter_map(|object| object.intersect(ray).map(|hit| (hit, object)))
            .min_by(|a, b| a.0.distance.partial_cmp(&b.0.distance).unwrap())
    }
}

const WIDTH: u32 = 1600;
const HEIGHT: u32 = 1200;

fn main() {
    let objects = vec![
        Object::sphere(
            Point3::new(-10.0, 0.0, 0.0),
            2.0,
            Vector3::new(1.0, 0.0, 0.0),
        ),
        Object::sphere(Point3::new(0.0, 0.0, 0.0), 5.0, Vector3::new(0.0, 1.0, 0.0)),
        Object::sphere(
            Point3::new(20.0, 0.0, 0.0),
            10.0,
            Vector3::new(0.0, 0.0, 1.0),
        ),
    ];

    let lights = vec![
        Light::new(Point3::new(-40.0, 20.0, 0.0), 0.8),
        Light::new(Point3::new(0.0, 20.0, -50.0), 0.4),
    ];

    let camera = Camera::new(Point3::new(-30.0, 30.0, -20.0), Point3::new(0.0, 0.0, 0.0));

    let scene = Scene::new(objects, camera, lights);

    let mut scene_buffer = [0; (WIDTH * HEIGHT * 3) as usize];

    let fov = std::f32::consts::PI / 4.0;
    let fov_adjust = f32::tan(fov / 2.0);

    let aspect_ratio = WIDTH as f32 / HEIGHT as f32;

    let camera_matrix = Matrix4::face_towards(
        &scene.camera.position,
        &scene.camera.look_at,
        &Vector3::new(0.0, 1.0, 0.0),
    );

    let start = std::time::Instant::now();

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let norm_x = (x as f32 + 0.5) / WIDTH as f32;
            let norm_y = (y as f32 + 0.5) / HEIGHT as f32;
            // Scale the x pixel according to the aspect ratio
            let screen_x = (2.0 * norm_x - 1.0) * aspect_ratio * fov_adjust;
            // Invert the y so +1 is at the top and -1 is at the bottom
            let screen_y = (1.0 - 2.0 * norm_y) * fov_adjust;

            let camera_point = Point4::new(screen_x, screen_y, 1.0, 1.0);

            let origin = camera_matrix * Point4::new(0.0, 0.0, 0.0, 1.0);
            let target = camera_matrix * camera_point;

            let direction = (target - origin).xyz().normalize();

            let ray = Ray::new(origin.xyz(), direction);
            let index = ((y * WIDTH + x) * 3) as usize;
            let color = scene.trace(&ray);

            scene_buffer[index]     = (color.x * 255.0) as u8;
            scene_buffer[index + 1] = (color.y * 255.0) as u8;
            scene_buffer[index + 2] = (color.z * 255.0) as u8;
        }
    }

    println!("Rendered frame in: {:?}", start.elapsed());

    // Output picture
    let path = Path::new("output.png");
    let file = File::create(path).unwrap();
    let writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(writer, WIDTH, HEIGHT);
    encoder.set_color(png::ColorType::RGB);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    writer.write_image_data(&scene_buffer).unwrap();
}
