//! Quaternion mathematics for spinor modulation.
//!
//! Unit quaternions represent rotations in 3D space and points on the 3-sphere S³.
//! This module provides the mathematical foundation for CSPM's 4D signal space.

use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Neg, Sub};

/// A quaternion q = w + xi + yj + zk
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quaternion {
    /// Create a new quaternion
    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Self { w, x, y, z }
    }

    /// Identity quaternion (1, 0, 0, 0)
    pub fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 0.0)
    }

    /// Zero quaternion
    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Create from array [w, x, y, z]
    pub fn from_array(arr: [f64; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }

    /// Convert to array [w, x, y, z]
    pub fn to_array(&self) -> [f64; 4] {
        [self.w, self.x, self.y, self.z]
    }

    /// Squared norm (w² + x² + y² + z²)
    pub fn norm_squared(&self) -> f64 {
        self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Norm (magnitude)
    pub fn norm(&self) -> f64 {
        self.norm_squared().sqrt()
    }

    /// Normalize to unit quaternion
    pub fn normalize(&self) -> Self {
        let n = self.norm();
        if n < 1e-10 {
            return Self::identity();
        }
        Self::new(self.w / n, self.x / n, self.y / n, self.z / n)
    }

    /// Check if normalized (unit quaternion)
    pub fn is_normalized(&self) -> bool {
        (self.norm_squared() - 1.0).abs() < 1e-6
    }

    /// Conjugate (w, -x, -y, -z)
    pub fn conjugate(&self) -> Self {
        Self::new(self.w, -self.x, -self.y, -self.z)
    }

    /// Inverse (conjugate / norm²)
    pub fn inverse(&self) -> Self {
        let n2 = self.norm_squared();
        if n2 < 1e-10 {
            return Self::identity();
        }
        let conj = self.conjugate();
        Self::new(conj.w / n2, conj.x / n2, conj.y / n2, conj.z / n2)
    }

    /// Dot product
    pub fn dot(&self, other: &Self) -> f64 {
        self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Euclidean distance between quaternions
    pub fn distance(&self, other: &Self) -> f64 {
        let dw = self.w - other.w;
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dw * dw + dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Angular distance on S³ (arc length)
    pub fn angular_distance(&self, other: &Self) -> f64 {
        let dot = self.dot(other).clamp(-1.0, 1.0);
        dot.abs().acos() * 2.0
    }

    /// Spherical linear interpolation (SLERP)
    pub fn slerp(&self, other: &Self, t: f64) -> Self {
        let mut dot = self.dot(other);

        // Ensure shortest path
        let other = if dot < 0.0 {
            dot = -dot;
            other.neg()
        } else {
            *other
        };

        // If very close, use linear interpolation
        if dot > 0.9995 {
            let result = *self + (other - *self) * t;
            return result.normalize();
        }

        let theta_0 = dot.acos();
        let theta = theta_0 * t;
        let sin_theta = theta.sin();
        let sin_theta_0 = theta_0.sin();

        let s0 = (theta_0 - theta).cos() - dot * sin_theta / sin_theta_0;
        let s1 = sin_theta / sin_theta_0;

        Self::new(
            self.w * s0 + other.w * s1,
            self.x * s0 + other.x * s1,
            self.y * s0 + other.y * s1,
            self.z * s0 + other.z * s1,
        )
    }

    /// Create quaternion from axis-angle representation
    pub fn from_axis_angle(axis: [f64; 3], angle: f64) -> Self {
        let half_angle = angle / 2.0;
        let s = half_angle.sin();
        let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();

        if norm < 1e-10 {
            return Self::identity();
        }

        Self::new(
            half_angle.cos(),
            axis[0] / norm * s,
            axis[1] / norm * s,
            axis[2] / norm * s,
        )
    }

    /// Rotate a 3D vector by this quaternion
    /// v' = q * v * q^(-1) where v = (0, vx, vy, vz)
    pub fn rotate_vector(&self, v: [f64; 3]) -> [f64; 3] {
        let qv = Quaternion::new(0.0, v[0], v[1], v[2]);
        let rotated = *self * qv * self.conjugate();
        [rotated.x, rotated.y, rotated.z]
    }

    /// Apply this rotation to another quaternion (sandwich product)
    /// q' = self * q * self^(-1)
    pub fn rotate_quaternion(&self, q: &Self) -> Self {
        *self * *q * self.inverse()
    }

    /// Check approximate equality
    pub fn approx_eq(&self, other: &Self, epsilon: f64) -> bool {
        (self.w - other.w).abs() < epsilon
            && (self.x - other.x).abs() < epsilon
            && (self.y - other.y).abs() < epsilon
            && (self.z - other.z).abs() < epsilon
    }

    /// Check if this quaternion represents the same rotation as another
    /// (quaternions q and -q represent the same rotation)
    pub fn same_rotation(&self, other: &Self, epsilon: f64) -> bool {
        self.approx_eq(other, epsilon) || self.approx_eq(&other.neg(), epsilon)
    }
}

// Operator implementations

impl Add for Quaternion {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(
            self.w + other.w,
            self.x + other.x,
            self.y + other.y,
            self.z + other.z,
        )
    }
}

impl Sub for Quaternion {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(
            self.w - other.w,
            self.x - other.x,
            self.y - other.y,
            self.z - other.z,
        )
    }
}

impl Mul for Quaternion {
    type Output = Self;

    /// Hamilton product
    fn mul(self, other: Self) -> Self {
        Self::new(
            self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        )
    }
}

impl Mul<f64> for Quaternion {
    type Output = Self;

    fn mul(self, scalar: f64) -> Self {
        Self::new(self.w * scalar, self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

impl Neg for Quaternion {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.w, -self.x, -self.y, -self.z)
    }
}

impl Default for Quaternion {
    fn default() -> Self {
        Self::identity()
    }
}

impl std::fmt::Display for Quaternion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:.4}, {:.4}, {:.4}, {:.4})", self.w, self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_identity() {
        let id = Quaternion::identity();
        assert!(id.is_normalized());
        assert_eq!(id.w, 1.0);
        assert_eq!(id.x, 0.0);
    }

    #[test]
    fn test_multiplication() {
        let q1 = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        let q2 = Quaternion::new(0.0, 1.0, 0.0, 0.0);

        // i * 1 = i
        let result = q2 * q1;
        assert!(result.approx_eq(&q2, 1e-10));

        // 1 * i = i
        let result = q1 * q2;
        assert!(result.approx_eq(&q2, 1e-10));
    }

    #[test]
    fn test_conjugate() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0).normalize();
        let conj = q.conjugate();

        // q * q* = |q|² * 1
        let product = q * conj;
        assert!(product.approx_eq(&Quaternion::identity(), 1e-10));
    }

    #[test]
    fn test_rotation() {
        // 90° rotation around z-axis
        let q = Quaternion::from_axis_angle([0.0, 0.0, 1.0], PI / 2.0);

        // Rotate x-axis vector -> should become y-axis
        let rotated = q.rotate_vector([1.0, 0.0, 0.0]);

        assert!((rotated[0] - 0.0).abs() < 1e-10);
        assert!((rotated[1] - 1.0).abs() < 1e-10);
        assert!((rotated[2] - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_slerp() {
        let q1 = Quaternion::identity();
        let q2 = Quaternion::from_axis_angle([0.0, 0.0, 1.0], PI);

        // Halfway should be 90° rotation
        let mid = q1.slerp(&q2, 0.5);
        let expected = Quaternion::from_axis_angle([0.0, 0.0, 1.0], PI / 2.0);

        assert!(mid.same_rotation(&expected, 1e-10));
    }

    #[test]
    fn test_distance() {
        let q1 = Quaternion::identity();
        let q2 = Quaternion::new(0.0, 1.0, 0.0, 0.0);

        // Distance between (1,0,0,0) and (0,1,0,0)
        let d = q1.distance(&q2);
        assert!((d - std::f64::consts::SQRT_2).abs() < 1e-10);
    }
}
