//! Quaternion algebra for 4D rotations
//!
//! Quaternions are the mathematical foundation of CSPM. Each vertex of the
//! 600-cell is a unit quaternion, and lattice rotations are quaternion multiplications.

use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Neg, Sub};

/// A quaternion q = w + xi + yj + zk
///
/// Unit quaternions (|q| = 1) represent 3D rotations and form
/// the vertices of the 600-cell in 4D space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quaternion {
    /// Scalar (real) component
    pub w: f64,
    /// i component
    pub x: f64,
    /// j component
    pub y: f64,
    /// k component
    pub z: f64,
}

impl Quaternion {
    /// Create a new quaternion
    pub const fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Self { w, x, y, z }
    }

    /// Identity quaternion (no rotation)
    pub const fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 0.0)
    }

    /// Zero quaternion
    pub const fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Create from axis-angle representation
    pub fn from_axis_angle(axis: [f64; 3], angle: f64) -> Self {
        let half_angle = angle / 2.0;
        let sin_half = half_angle.sin();
        let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();

        if norm < 1e-10 {
            return Self::identity();
        }

        Self {
            w: half_angle.cos(),
            x: axis[0] / norm * sin_half,
            y: axis[1] / norm * sin_half,
            z: axis[2] / norm * sin_half,
        }
    }

    /// Squared magnitude
    #[inline]
    pub fn norm_squared(&self) -> f64 {
        self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Magnitude (norm)
    #[inline]
    pub fn norm(&self) -> f64 {
        self.norm_squared().sqrt()
    }

    /// Normalize to unit quaternion
    pub fn normalize(&self) -> Self {
        let n = self.norm();
        if n < 1e-10 {
            return Self::identity();
        }
        Self {
            w: self.w / n,
            x: self.x / n,
            y: self.y / n,
            z: self.z / n,
        }
    }

    /// Conjugate: q* = w - xi - yj - zk
    #[inline]
    pub fn conjugate(&self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    /// Inverse: q^(-1) = q* / |q|²
    pub fn inverse(&self) -> Self {
        let n2 = self.norm_squared();
        if n2 < 1e-10 {
            return Self::identity();
        }
        Self {
            w: self.w / n2,
            x: -self.x / n2,
            y: -self.y / n2,
            z: -self.z / n2,
        }
    }

    /// Dot product (inner product in R⁴)
    #[inline]
    pub fn dot(&self, other: &Self) -> f64 {
        self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Geodesic distance on the 3-sphere
    ///
    /// This is the natural distance metric for unit quaternions.
    /// For 600-cell snapping, we want minimum geodesic distance.
    pub fn geodesic_distance(&self, other: &Self) -> f64 {
        let dot = self.dot(other).abs().min(1.0); // Clamp for numerical stability
        2.0 * (1.0 - dot * dot).sqrt().asin()
    }

    /// Chordal distance (faster, monotonic with geodesic)
    ///
    /// For nearest-neighbor lookup, chordal distance gives same result
    /// but avoids expensive asin computation.
    #[inline]
    pub fn chordal_distance_squared(&self, other: &Self) -> f64 {
        let dot = self.dot(other).abs();
        2.0 * (1.0 - dot)
    }

    /// Rotate a vector by this quaternion: v' = q v q*
    pub fn rotate_vector(&self, v: [f64; 3]) -> [f64; 3] {
        // Treat vector as pure quaternion (w=0)
        let p = Quaternion::new(0.0, v[0], v[1], v[2]);
        let rotated = *self * p * self.conjugate();
        [rotated.x, rotated.y, rotated.z]
    }

    /// Rotate another quaternion: q' = self * q * self*
    ///
    /// This is the lattice rotation operation in CSPM.
    pub fn rotate_quaternion(&self, q: &Self) -> Self {
        *self * *q * self.conjugate()
    }

    /// Linear interpolation (not normalized - use slerp for unit quaternions)
    pub fn lerp(&self, other: &Self, t: f64) -> Self {
        Self {
            w: self.w + t * (other.w - self.w),
            x: self.x + t * (other.x - self.x),
            y: self.y + t * (other.y - self.y),
            z: self.z + t * (other.z - self.z),
        }
    }

    /// Spherical linear interpolation (for unit quaternions)
    pub fn slerp(&self, other: &Self, t: f64) -> Self {
        let mut dot = self.dot(other);

        // If negative dot, negate one quaternion (shortest path)
        let other = if dot < 0.0 {
            dot = -dot;
            other.neg()
        } else {
            *other
        };

        // If very close, use linear interpolation
        if dot > 0.9995 {
            return self.lerp(&other, t).normalize();
        }

        let theta = dot.acos();
        let sin_theta = theta.sin();

        let s1 = ((1.0 - t) * theta).sin() / sin_theta;
        let s2 = (t * theta).sin() / sin_theta;

        Self {
            w: s1 * self.w + s2 * other.w,
            x: s1 * self.x + s2 * other.x,
            y: s1 * self.y + s2 * other.y,
            z: s1 * self.z + s2 * other.z,
        }
    }
}

impl Mul for Quaternion {
    type Output = Self;

    /// Hamilton product: quaternion multiplication
    fn mul(self, rhs: Self) -> Self {
        Self {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }
}

impl Mul<f64> for Quaternion {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self {
            w: self.w * rhs,
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Add for Quaternion {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            w: self.w + rhs.w,
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for Quaternion {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            w: self.w - rhs.w,
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Neg for Quaternion {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            w: -self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    fn quat_approx_eq(a: &Quaternion, b: &Quaternion) -> bool {
        approx_eq(a.w, b.w) && approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z)
    }

    #[test]
    fn test_identity() {
        let id = Quaternion::identity();
        assert!(approx_eq(id.norm(), 1.0));
    }

    #[test]
    fn test_multiplication() {
        let i = Quaternion::new(0.0, 1.0, 0.0, 0.0);
        let j = Quaternion::new(0.0, 0.0, 1.0, 0.0);
        let k = Quaternion::new(0.0, 0.0, 0.0, 1.0);

        // i*j = k
        let ij = i * j;
        assert!(quat_approx_eq(&ij, &k));

        // j*k = i
        let jk = j * k;
        assert!(quat_approx_eq(&jk, &i));

        // i*i = -1
        let ii = i * i;
        assert!(quat_approx_eq(&ii, &Quaternion::new(-1.0, 0.0, 0.0, 0.0)));
    }

    #[test]
    fn test_normalize() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let n = q.normalize();
        assert!(approx_eq(n.norm(), 1.0));
    }

    #[test]
    fn test_conjugate() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0);
        let qc = q.conjugate();
        assert_eq!(qc.w, q.w);
        assert_eq!(qc.x, -q.x);
        assert_eq!(qc.y, -q.y);
        assert_eq!(qc.z, -q.z);
    }

    #[test]
    fn test_inverse() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0).normalize();
        let qi = q.inverse();
        let product = q * qi;
        assert!(quat_approx_eq(&product, &Quaternion::identity()));
    }

    #[test]
    fn test_rotation() {
        // 90° rotation around z-axis
        let q = Quaternion::from_axis_angle([0.0, 0.0, 1.0], std::f64::consts::FRAC_PI_2);
        let v = [1.0, 0.0, 0.0];
        let rotated = q.rotate_vector(v);

        assert!(approx_eq(rotated[0], 0.0));
        assert!(approx_eq(rotated[1], 1.0));
        assert!(approx_eq(rotated[2], 0.0));
    }

    #[test]
    fn test_geodesic_distance() {
        let a = Quaternion::identity();
        let b = Quaternion::new(0.0, 1.0, 0.0, 0.0); // 180° rotation

        let dist = a.geodesic_distance(&b);
        assert!(approx_eq(dist, std::f64::consts::PI));
    }
}
