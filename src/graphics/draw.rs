//!

use super::{Color, LinearColor, Rect};

/// A struct that represents where to put a `Drawable`.
///
/// This can either be a set of individual components, or
/// a single `Matrix4` transform.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Transform {
    /// Transform made of individual values
    Values {
        /// The position to draw the graphic expressed as a `Point2`.
        dest: mint::Point2<f32>,
        /// The orientation of the graphic in radians.
        rotation: f32,
        /// The x/y scale factors expressed as a `Vector2`.
        scale: mint::Vector2<f32>,
        /// An offset, which is applied before scaling and rotation happen.
        offset: mint::Point2<f32>,
    },
    /// Transform made of an arbitrary matrix.
    ///
    /// It should represent the final model matrix of the given drawable.  This is useful for
    /// situations where, for example, you build your own hierarchy system, where you calculate
    /// matrices of each hierarchy item and store a calculated world-space model matrix of an item.
    /// This lets you implement transform stacks, skeletal animations, etc.
    Matrix(mint::ColumnMatrix4<f32>),
}

impl Default for Transform {
    fn default() -> Self {
        Transform::Values {
            dest: mint::Point2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            scale: mint::Vector2 { x: 1.0, y: 1.0 },
            offset: mint::Point2 { x: 0.0, y: 0.0 },
        }
    }
}

impl Transform {
    /// Crunches the transform down to a single matrix, if it's not one already.
    pub fn to_matrix(self) -> Self {
        Transform::Matrix(self.to_bare_matrix())
    }

    /// Same as `to_matrix()` but just returns a bare `mint` matrix.
    pub fn to_bare_matrix(self) -> mint::ColumnMatrix4<f32> {
        match self {
            Transform::Matrix(m) => m,
            Transform::Values {
                dest,
                rotation,
                scale,
                offset,
            } => {
                // Calculate a matrix equivalent to doing this:
                // type Vec3 = na::Vector3<f32>;
                // let o = offset;
                // let translate = na::Matrix4::new_translation(&Vec3::new(dest.x, dest.y, 0.0));
                // let offset = na::Matrix4::new_translation(&Vec3::new(offset.x, offset.y, 0.0));
                // let offset_inverse =
                //     na::Matrix4::new_translation(&Vec3::new(-o.x, -o.y, 0.0));
                // let axis_angle = Vec3::z() * *rotation;
                // let rotation = na::Matrix4::new_rotation(axis_angle);
                // let scale = na::Matrix4::new_nonuniform_scaling(&Vec3::new(scale.x, scale.y, 1.0));
                // translate * rotation * scale * offset_inverse
                //
                // Doing the bits manually is faster though, or at least was last I checked.
                let (sinr, cosr) = rotation.sin_cos();
                let m00 = cosr * scale.x;
                let m01 = -sinr * scale.y;
                let m10 = sinr * scale.x;
                let m11 = cosr * scale.y;
                let m03 = offset.x * (-m00) - offset.y * m01 + dest.x;
                let m13 = offset.y * (-m11) - offset.x * m10 + dest.y;
                // Welp, this transpose fixes some bug that makes nothing draw,
                // that was introduced in commit 2c6b3cc03f34fb240f4246f5a68c75bd85b60eae.
                // The best part is, I don't know if this code is wrong, or whether there's
                // some reversed matrix multiply or such somewhere else that this cancel
                // out.  Probably the former though.
                glam::Mat4::from_cols_array(&[
                    m00, m01, 0.0, m03, // oh rustfmt you so fine
                    m10, m11, 0.0, m13, // you so fine you blow my mind
                    0.0, 0.0, 1.0, 0.0, // but leave my matrix formatting alone
                    0.0, 0.0, 0.0, 1.0, // plz
                ])
                .transpose()
                .into()
            }
        }
    }
}

/// Value describing the Z "coordinate" of a draw.
///
/// Greater values correspond to the foreground, and lower values
/// correspond to the background.
pub type ZIndex = i32;

/// A struct containing all the necessary info for drawing with parameters.
///
/// This struct implements the `Default` trait, so to set only some parameter
/// you can just do:
///
/// ```rust
/// # use ggez::*;
/// # use ggez::graphics::*;
/// # fn t<P>(ctx: &mut Context, drawable: &P) where P: Drawable {
/// let my_dest = glam::vec2(13.0, 37.0);
/// graphics::draw(ctx, drawable, DrawParam::default().dest(my_dest) );
/// # }
/// ```
///
/// As a shortcut, it also implements `From` for a variety of tuple types.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DrawParam {
    /// A portion of the drawable to clip, as a fraction of the whole image.
    /// Defaults to the whole image `(0,0 to 1,1)` if omitted.
    pub src: Rect,
    /// Default: white.
    pub color: Color,
    /// Where to put the `Drawable`.
    pub transform: Transform,
    /// Whether the scale should be relative to image size.
    pub image_scale: bool,
    /// The Z coordinate of the draw.
    ///
    /// This has no effect when passed into [`InstanceArray`].
    pub z: ZIndex,
}

impl Default for DrawParam {
    fn default() -> Self {
        DrawParam {
            src: Rect::one(),
            color: Color::WHITE,
            transform: Transform::default(),
            image_scale: true,
            z: 0,
        }
    }
}

impl DrawParam {
    /// Create a new DrawParam with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the source rect.
    pub fn src(mut self, src: Rect) -> Self {
        self.src = src;
        self
    }

    /// Set the dest point.
    pub fn dest<P>(mut self, dest_: P) -> Self
    where
        P: Into<mint::Point2<f32>>,
    {
        if let Transform::Values { ref mut dest, .. } = self.transform {
            let p: mint::Point2<f32> = dest_.into();
            *dest = p;
            self
        } else {
            panic!("Cannot set values for a DrawParam matrix")
        }
    }

    /// Set the `dest` and `scale` together.
    pub fn dest_rect(self, rect: Rect) -> Self {
        self.dest(rect.point()).scale(rect.size())
    }

    /// Set the color. This will be blended with whatever
    /// color the drawn object already is.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }

    /// Set the rotation.
    pub fn rotation(mut self, rot: f32) -> Self {
        if let Transform::Values {
            ref mut rotation, ..
        } = self.transform
        {
            *rotation = rot;
            self
        } else {
            panic!("Cannot set values for a DrawParam matrix")
        }
    }

    /// Set the scaling factors.
    pub fn scale<V>(mut self, scale_: V) -> Self
    where
        V: Into<mint::Vector2<f32>>,
    {
        if let Transform::Values { ref mut scale, .. } = self.transform {
            let p: mint::Vector2<f32> = scale_.into();
            *scale = p;
            self
        } else {
            panic!("Cannot set values for a DrawParam matrix")
        }
    }

    /// Set the transformation offset.
    pub fn offset<P>(mut self, offset_: P) -> Self
    where
        P: Into<mint::Point2<f32>>,
    {
        if let Transform::Values { ref mut offset, .. } = self.transform {
            let p: mint::Point2<f32> = offset_.into();
            *offset = p;
            self
        } else {
            panic!("Cannot set values for a DrawParam matrix")
        }
    }

    /// Set the transformation matrix.
    pub fn transform<M>(mut self, transform: M) -> Self
    where
        M: Into<mint::ColumnMatrix4<f32>>,
    {
        self.transform = Transform::Matrix(transform.into());
        self
    }

    /// Set the image scale option.
    pub fn image_scale(mut self, image_scale: bool) -> Self {
        self.image_scale = image_scale;
        self
    }

    /// Set the Z coordinate.
    pub fn z(mut self, z: ZIndex) -> Self {
        self.z = z;
        self
    }
}

/// Create a `DrawParam` from a location.
/// Note that this takes a single-element tuple.
/// It's a little weird but keeps the trait implementations
/// from clashing.
impl<P> From<(P,)> for DrawParam
where
    P: Into<mint::Point2<f32>>,
{
    fn from(location: (P,)) -> Self {
        DrawParam::new().dest(location.0)
    }
}

/// Create a `DrawParam` from a location and color
impl<P> From<(P, Color)> for DrawParam
where
    P: Into<mint::Point2<f32>>,
{
    fn from((location, color): (P, Color)) -> Self {
        DrawParam::new().dest(location).color(color)
    }
}

/// Create a `DrawParam` from a location, rotation and color
impl<P> From<(P, f32, Color)> for DrawParam
where
    P: Into<mint::Point2<f32>>,
{
    fn from((location, rotation, color): (P, f32, Color)) -> Self {
        DrawParam::new()
            .dest(location)
            .rotation(rotation)
            .color(color)
    }
}

/// Create a `DrawParam` from a location, rotation, offset and color
impl<P> From<(P, f32, P, Color)> for DrawParam
where
    P: Into<mint::Point2<f32>>,
{
    fn from((location, rotation, offset, color): (P, f32, P, Color)) -> Self {
        DrawParam::new()
            .dest(location)
            .rotation(rotation)
            .offset(offset)
            .color(color)
    }
}

/// Create a `DrawParam` from a location, rotation, offset, scale and color
impl<P, V> From<(P, f32, P, V, Color)> for DrawParam
where
    P: Into<mint::Point2<f32>>,
    V: Into<mint::Vector2<f32>>,
{
    fn from((location, rotation, offset, scale, color): (P, f32, P, V, Color)) -> Self {
        DrawParam::new()
            .dest(location)
            .rotation(rotation)
            .offset(offset)
            .scale(scale)
            .color(color)
    }
}

#[derive(crevice::std140::AsStd140)]
pub(crate) struct DrawUniforms {
    pub color: mint::Vector4<f32>,
    pub src_rect: mint::Vector4<f32>,
    pub transform: mint::ColumnMatrix4<f32>,
}

impl DrawUniforms {
    pub fn from_param(param: DrawParam, mut image_scale: mint::Vector2<f32>) -> Self {
        if !param.image_scale {
            image_scale.x = 1.;
            image_scale.y = 1.;
        }

        let scale_x = param.src.w * image_scale.x;
        let scale_y = param.src.h * image_scale.y;
        let param = match param.transform {
            Transform::Values { scale, .. } => param.scale(mint::Vector2 {
                x: scale.x * scale_x,
                y: scale.y * scale_y,
            }),
            Transform::Matrix(m) => param.transform(
                glam::Mat4::from(m) * glam::Mat4::from_scale(glam::vec3(scale_x, scale_y, 1.)),
            ),
        };

        let color = LinearColor::from(param.color);

        DrawUniforms {
            color: <[f32; 4]>::from(color).into(),
            src_rect: mint::Vector4 {
                x: param.src.x,
                y: param.src.y,
                z: param.src.x + param.src.w,
                w: param.src.y + param.src.h,
            },
            transform: param.transform.to_bare_matrix(),
        }
    }
}
