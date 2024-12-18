use cubecl_core as cubecl;
use cubecl_core::prelude::*;

use crate::matmul::components::global::tensor_view::TensorWriter;
use crate::matmul::components::global::tilewise_unloading::TilewiseUnloading;
use crate::matmul::components::stage::StageWriter;
use crate::tensor::ReadWrite;
use crate::{matmul::components::global, tensor::VirtualTensor};

#[derive(CubeType)]
pub struct Unloader<EG: Numeric> {
    pub tensor_view: TensorWriter<EG>,
}

#[cube]
impl<EG: Numeric> global::Unloader<EG> for Unloader<EG> {
    type StageWriter = Self;

    fn as_stage_writer<G: global::Config>(this: Self) -> Self::StageWriter {
        this
    }
}

#[cube]
impl<EG: Numeric> Unloader<EG> {
    pub fn new(
        tensor: VirtualTensor<EG, ReadWrite>,
        x_offset: u32,
        y_offset: u32,
        batch_offset: u32,
    ) -> Self {
        Unloader::<EG> {
            tensor_view: TensorWriter::new(tensor, x_offset, y_offset, batch_offset),
        }
    }
}

#[cube]
impl<EG: Numeric> StageWriter<EG> for Unloader<EG> {
    fn write<ES: Numeric, G: global::Config>(
        this: &mut Self,
        slice: Slice<Line<ES>>,
        compute_plane_offset: u32,
        accumulator_offset: u32,
        #[comptime] config: G,
    ) {
        TilewiseUnloading::unload_from_slice::<EG, ES, G>(
            &mut this.tensor_view,
            slice,
            compute_plane_offset,
            accumulator_offset,
            config,
        );
    }
}
