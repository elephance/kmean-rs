use crate::api::DistanceFunction;
use crate::memory::*;
use crate::{KMeans, KMeansConfig, KMeansState};
use rand::prelude::*;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::simd::{LaneCount, Simd, SupportedLaneCount};

#[inline(always)]
pub fn calculate<T, const LANES: usize, D>(kmean: &KMeans<T, LANES, D>, state: &mut KMeansState<T>, config: &KMeansConfig<'_, T>)
where
    T: Primitive,
    LaneCount<LANES>: SupportedLaneCount,
    Simd<T, LANES>: SupportedSimdArray<T, LANES>,
    D: DistanceFunction<T, LANES>,
{
    let (assignments, centroids, centroid_frequency, k) =
        (&mut state.assignments, &mut state.centroids, &mut state.centroid_frequency, state.k);

    assignments.iter_mut().for_each(|a| {
        *a = config.rnd.borrow_mut().gen_range(0..k);
        centroid_frequency[*a] += 1;
    });
    kmean.p_samples.iter().for_each(|sb| {
        sb.chunks_exact_stride()
            .zip(assignments.iter().cloned())
            .for_each(|(sample, assignment)| {
                centroids
                    .bfr
                    .iter_mut()
                    .skip(centroids.stride * assignment)
                    .zip(sample.iter().cloned())
                    .for_each(|(cv, sv)| *cv += sv / T::from(centroid_frequency[assignment]).unwrap());
            });
    });
}
