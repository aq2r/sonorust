pub mod dict;
pub mod help;
pub mod join;
pub mod leave;
pub mod length;
pub mod model;
pub mod now;
pub mod ping;
pub mod reload;
pub mod server;
pub mod speaker;
pub mod style;
pub mod wav;

pub use dict::dict;
pub use help::help;
pub use join::join;
pub use leave::leave;
pub use length::length;
pub use model::model;
pub use now::now;
pub use reload::reload;
pub use server::server;
pub use speaker::speaker;
pub use style::style;
pub use wav::wav;

pub enum Either<L, R>
where
    L: Sync + Send,
    R: Sync + Send,
{
    Left(L),
    Right(R),
}
