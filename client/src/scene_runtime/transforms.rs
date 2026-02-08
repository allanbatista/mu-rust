use crate::scene_runtime::scene_loader::SceneRotationEncoding;
use bevy::math::{Mat3, Quat};

/// Converts a scene-object rotation payload into a Bevy quaternion.
///
/// The converter historically emitted a swizzled Euler payload (`[x, z, y]`).
/// New assets can emit raw MU angles (`[x, y, z]`) and select the encoding via
/// `scene_objects.json.metadata.rotation_encoding`.
pub fn scene_object_rotation_to_quat(
    rotation_degrees: [f32; 3],
    rotation_encoding: SceneRotationEncoding,
) -> Quat {
    let mu_angles = match rotation_encoding {
        SceneRotationEncoding::LegacySwizzledDegrees => [
            rotation_degrees[0],
            rotation_degrees[2],
            rotation_degrees[1],
        ],
        SceneRotationEncoding::MuAnglesDegrees => rotation_degrees,
    };
    mu_angles_degrees_to_bevy_quat(mu_angles)
}

/// Converts MU map-object angles (degrees, `AngleMatrix` convention) into Bevy quaternions.
pub fn mu_angles_degrees_to_bevy_quat(mu_angles_degrees: [f32; 3]) -> Quat {
    let mu_matrix = mu_angle_matrix(mu_angles_degrees);
    let bevy_matrix = mu_to_bevy_basis(mu_matrix);
    let bevy_mat3 = mat3_from_row_major(bevy_matrix);
    let rotation = Quat::from_mat3(&bevy_mat3);
    if rotation.is_finite() {
        rotation.normalize()
    } else {
        Quat::IDENTITY
    }
}

/// Port of MU's `AngleMatrix` (`Math/ZzzMathLib.cpp`):
/// matrix = (Z * Y) * X using degrees.
fn mu_angle_matrix(angles: [f32; 3]) -> [[f32; 3]; 3] {
    let angle_z = angles[2].to_radians();
    let sy = angle_z.sin();
    let cy = angle_z.cos();

    let angle_y = angles[1].to_radians();
    let sp = angle_y.sin();
    let cp = angle_y.cos();

    let angle_x = angles[0].to_radians();
    let sr = angle_x.sin();
    let cr = angle_x.cos();

    [
        [cp * cy, sr * sp * cy + cr * -sy, cr * sp * cy + -sr * -sy],
        [cp * sy, sr * sp * sy + cr * cy, cr * sp * sy + -sr * cy],
        [-sp, sr * cp, cr * cp],
    ]
}

/// Converts MU basis (X,Y,Z-up) into Bevy basis (X,Z,Y-up).
fn mu_to_bevy_basis(mu_matrix: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    [
        [mu_matrix[0][0], mu_matrix[0][2], mu_matrix[0][1]],
        [mu_matrix[2][0], mu_matrix[2][2], mu_matrix[2][1]],
        [mu_matrix[1][0], mu_matrix[1][2], mu_matrix[1][1]],
    ]
}

fn mat3_from_row_major(matrix: [[f32; 3]; 3]) -> Mat3 {
    Mat3::from_cols_array(&[
        matrix[0][0],
        matrix[1][0],
        matrix[2][0],
        matrix[0][1],
        matrix[1][1],
        matrix[2][1],
        matrix[0][2],
        matrix[1][2],
        matrix[2][2],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn legacy_and_mu_encodings_produce_the_same_rotation() {
        let mu_angles = [12.0, -37.0, 146.0];
        let legacy_swizzled = [mu_angles[0], mu_angles[2], mu_angles[1]];

        let from_mu =
            scene_object_rotation_to_quat(mu_angles, SceneRotationEncoding::MuAnglesDegrees);
        let from_legacy = scene_object_rotation_to_quat(
            legacy_swizzled,
            SceneRotationEncoding::LegacySwizzledDegrees,
        );

        assert_quat_eq(from_mu, from_legacy);
    }

    #[test]
    fn mu_yaw_rotates_around_bevy_up_axis_after_basis_conversion() {
        let rotation = mu_angles_degrees_to_bevy_quat([0.0, 0.0, 90.0]);
        let rotated_forward = rotation * Vec3::Z;
        assert_vec3_eq(rotated_forward, -Vec3::X);
    }

    #[test]
    fn converted_quaternion_matches_mu_matrix_reference() {
        let test_angles = [
            [0.0, 0.0, 0.0],
            [10.0, 20.0, 30.0],
            [-45.0, 90.0, 180.0],
            [5.5, -122.0, 271.0],
        ];
        let test_vectors = [
            Vec3::X,
            Vec3::Y,
            Vec3::Z,
            Vec3::new(0.3, -0.7, 0.2).normalize(),
        ];

        for angles in test_angles {
            let rotation = mu_angles_degrees_to_bevy_quat(angles);
            let mu_matrix = mu_angle_matrix(angles);

            for vector in test_vectors {
                let expected = mu_rotate_then_swizzle(mu_matrix, vector);
                let actual = rotation * vector;
                assert_vec3_eq(actual, expected);
            }
        }
    }

    fn mu_rotate_then_swizzle(mu_matrix: [[f32; 3]; 3], bevy_vector: Vec3) -> Vec3 {
        let mu_vector = bevy_to_mu(bevy_vector);
        let rotated_mu = apply_row_major(mu_matrix, mu_vector);
        mu_to_bevy(rotated_mu)
    }

    fn apply_row_major(matrix: [[f32; 3]; 3], vector: Vec3) -> Vec3 {
        Vec3::new(
            matrix[0][0] * vector.x + matrix[0][1] * vector.y + matrix[0][2] * vector.z,
            matrix[1][0] * vector.x + matrix[1][1] * vector.y + matrix[1][2] * vector.z,
            matrix[2][0] * vector.x + matrix[2][1] * vector.y + matrix[2][2] * vector.z,
        )
    }

    fn mu_to_bevy(vector: Vec3) -> Vec3 {
        Vec3::new(vector.x, vector.z, vector.y)
    }

    fn bevy_to_mu(vector: Vec3) -> Vec3 {
        Vec3::new(vector.x, vector.z, vector.y)
    }

    fn assert_vec3_eq(actual: Vec3, expected: Vec3) {
        let delta = actual - expected;
        assert!(
            delta.length() <= EPSILON,
            "expected {:?}, got {:?}, delta={:?}",
            expected,
            actual,
            delta
        );
    }

    fn assert_quat_eq(actual: Quat, expected: Quat) {
        let dot = actual.dot(expected).abs();
        assert!(
            (1.0 - dot) <= EPSILON,
            "expected {:?}, got {:?}, dot={dot}",
            expected,
            actual
        );
    }
}
