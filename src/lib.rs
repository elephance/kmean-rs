#![feature(test)]
#![feature(portable_simd)]

//! # kmeans - API documentation
//!
//! Kmeans is a small rust library for the calculation of k-means-clustering.
//!
//! ## Design target
//! It's main target is high performance / throughput, you will therefore find
//! most of its API-surface rather plain.
//! An example of this is, that samples are given using a raw vector, instead of
//! any high-level arithmetics / matrix crate such as nalgebra or ndarray.
//! For this same reason, the crate is internally using hand-written SIMD operations
//! and (unfortunately) quite a bit of unsafe code, which proved to lead to insane speedups.
//!
//! ## Supported variants
//! K-Means clustering is not one algorithm, but more like a concept describing the outcome.
//! There are differing implementations and variants using differing levels of approximations.
//! For a list of supported variants, have a look at the documentation of [`KMeans`].
//!
//! ## Supported centroid initializations
//! The outcome of each K-Means run depends on the initialization of its clusters. There exist
//! multiple algorithms for this initialization, most of which are based on at least some
//! degree of randomness. For a list of implemented initialization methods, see [`KMeans`].
//!
//! ## Supported primitive types
//! - [`f32`]
//! - [`f64`]
//!
//! ## Example
//! Both: Supported variants and supported centroid initializations can be combined at will.
//! Here is an example showing the default full k-Means implementation, using K-Mean++ initialization:
//!
//! ```rust
//! use kmeans::*;
//!
//! let (sample_cnt, sample_dims, k, max_iter) = (20000, 200, 4, 100);
//!
//! // Generate some random data
//! let mut samples = vec![0.0f64;sample_cnt * sample_dims];
//! samples.iter_mut().for_each(|v| *v = rand::random());
//!
//! // Calculate kmeans, using kmean++ as initialization-method
//! // KMeans<_, 8> specifies to use f64 SIMD vectors with 8 lanes (e.g. AVX512)
//! let kmean: KMeans<_, 8, _> = KMeans::new(&samples, sample_cnt, sample_dims, EuclideanDistance);
//! let result = kmean.kmeans_lloyd(k, max_iter, KMeans::init_kmeanplusplus, &KMeansConfig::default());
//!
//! println!("Centroids: {:?}", result.centroids);
//! println!("Cluster-Assignments: {:?}", result.assignments);
//! println!("Error: {}", result.distsum);
//! ```
//!
//! ## Example (using the status event callbacks)
//! ```rust
//! use kmeans::*;
//!
//! let (sample_cnt, sample_dims, k, max_iter) = (20000, 200, 4, 2500);
//!
//! // Generate some random data
//! let mut samples = vec![0.0f64;sample_cnt * sample_dims];
//! samples.iter_mut().for_each(|v| *v = rand::random());
//!
//! let conf = KMeansConfig::build()
//!     .init_done(&|_| println!("Initialization completed."))
//!     .iteration_done(&|s, nr, new_distsum|
//!         println!("Iteration {} - Error: {:.2} -> {:.2} | Improvement: {:.2}",
//!             nr, s.distsum, new_distsum, s.distsum - new_distsum))
//!     .build();
//!
//! // Calculate kmeans, using kmean++ as initialization-method
//! // KMeans<_, 8> specifies to use f64 SIMD vectors with 8 lanes (e.g. AVX512)
//! let kmean: KMeans<_, 8, _> = KMeans::new(&samples, sample_cnt, sample_dims, EuclideanDistance);
//! let result = kmean.kmeans_minibatch(4, k, max_iter, KMeans::init_random_sample, &conf);
//!
//! println!("Centroids: {:?}", result.centroids);
//! println!("Cluster-Assignments: {:?}", result.assignments);
//! println!("Error: {}", result.distsum);
//! ```
//!
//! ## Short API-Overview / Description
//! Entry-point of the library is the [`KMeans`] struct. This struct is generic over the underlying primitive
//! type, that should be used for the calculations. To use KMeans, an instance of this struct is created, taking
//! over the sample data into its ownership (and doing some memory-related optimizations).
//!
//! **Note**: The input-data has to use the same primitive as the required output-data (distances).
//!
//! The [`KMeans`] struct's instance-methods represent the supported k-Means variants & implementations.
//! Calling such a method (e.g. [`KMeans::kmeans_lloyd`]) on the struct does not mutate it, so multiple runs can be
//! done in parallel (the algorithm itself is already parallellized though). Internally, a new instance of
//! [`KMeansState`] is used to store the state (and finally the result) of a K-Means calculation.
//!
//! All of the instance-methods take multiple arguments. One of which is the chosen centroid initialization method. These
//! initialization-method implementations are static methods within the [`KMeans`] struct, which are simply passed in as reference.

extern crate test;
#[macro_use]
mod helpers;
mod abort_strategy;
mod api;
mod distances;
mod inits;
mod memory;
mod variants;

pub use abort_strategy::AbortStrategy;
pub use api::{DistanceFunction, KMeans, KMeansConfig, KMeansConfigBuilder, KMeansState};
pub use distances::{EuclideanDistance, HistogramDistance};
pub use memory::Primitive;

#[cfg(test)]
mod tests {
    use super::*;
    use distances::EuclideanDistance;
    use memory::SupportedSimdArray;
    use rand::prelude::*;
    use std::simd::{LaneCount, Simd, SupportedLaneCount};
    use test::Bencher;

    #[bench]
    fn complete_benchmark_lloyd_small_f64x8(b: &mut Bencher) { complete_benchmark_lloyd::<f64, 8>(b, 200, 2000, 10, 32); }
    #[bench]
    fn complete_benchmark_lloyd_mid_f64x8(b: &mut Bencher) { complete_benchmark_lloyd::<f64, 8>(b, 2000, 200, 10, 32); }
    #[bench]
    fn complete_benchmark_lloyd_big_f64x8(b: &mut Bencher) { complete_benchmark_lloyd::<f64, 8>(b, 10000, 8, 10, 32); }
    #[bench]
    fn complete_benchmark_lloyd_huge_f64x8(b: &mut Bencher) { complete_benchmark_lloyd::<f64, 8>(b, 20000, 256, 1, 32); }

    #[bench]
    fn complete_benchmark_lloyd_small_f32x8(b: &mut Bencher) { complete_benchmark_lloyd::<f32, 8>(b, 200, 2000, 10, 32); }
    #[bench]
    fn complete_benchmark_lloyd_mid_f32x8(b: &mut Bencher) { complete_benchmark_lloyd::<f32, 8>(b, 2000, 200, 10, 32); }
    #[bench]
    fn complete_benchmark_lloyd_big_f32x8(b: &mut Bencher) { complete_benchmark_lloyd::<f32, 8>(b, 10000, 8, 10, 32); }
    #[bench]
    fn complete_benchmark_lloyd_huge_f32x8(b: &mut Bencher) { complete_benchmark_lloyd::<f32, 8>(b, 20000, 256, 1, 32); }

    fn complete_benchmark_lloyd<T: Primitive, const LANES: usize>(
        b: &mut Bencher, sample_cnt: usize, sample_dims: usize, max_iter: usize, k: usize,
    ) where
        T: Primitive,
        LaneCount<LANES>: SupportedLaneCount,
        Simd<T, LANES>: SupportedSimdArray<T, LANES>,
    {
        let mut rnd = rand::rngs::StdRng::seed_from_u64(1337);
        let mut samples = vec![T::zero(); sample_cnt * sample_dims];
        samples.iter_mut().for_each(|v| *v = rnd.gen_range(T::zero()..T::one()));
        let kmean: KMeans<T, LANES, _> = KMeans::new(&samples, sample_cnt, sample_dims, EuclideanDistance);
        let conf = KMeansConfig::build().random_generator(rnd).build();
        b.iter(|| kmean.kmeans_lloyd(k, max_iter, KMeans::init_kmeanplusplus, &conf));
    }

    #[bench]
    fn complete_benchmark_minibatch_small_f64x8(b: &mut Bencher) { complete_benchmark_minibatch::<f64, 8>(b, 30, 200, 2000, 100, 32); }
    #[bench]
    fn complete_benchmark_minibatch_mid_f64x8(b: &mut Bencher) { complete_benchmark_minibatch::<f64, 8>(b, 200, 2000, 200, 100, 32); }
    #[bench]
    fn complete_benchmark_minibatch_big_f64x8(b: &mut Bencher) { complete_benchmark_minibatch::<f64, 8>(b, 1000, 10000, 8, 100, 32); }
    #[bench]
    fn complete_benchmark_minibatch_huge_f64x8(b: &mut Bencher) { complete_benchmark_minibatch::<f64, 8>(b, 2000, 20000, 256, 30, 32); }
    #[bench]
    fn complete_benchmark_minibatch_small_f32x8(b: &mut Bencher) { complete_benchmark_minibatch::<f32, 8>(b, 30, 200, 2000, 100, 32); }
    #[bench]
    fn complete_benchmark_minibatch_mid_f32x8(b: &mut Bencher) { complete_benchmark_minibatch::<f32, 8>(b, 200, 2000, 200, 100, 32); }
    #[bench]
    fn complete_benchmark_minibatch_big_f32x8(b: &mut Bencher) { complete_benchmark_minibatch::<f32, 8>(b, 1000, 10000, 8, 100, 32); }
    #[bench]
    fn complete_benchmark_minibatch_huge_f32x8(b: &mut Bencher) { complete_benchmark_minibatch::<f32, 8>(b, 2000, 20000, 256, 30, 32); }

    fn complete_benchmark_minibatch<T: Primitive, const LANES: usize>(
        b: &mut Bencher, batch_size: usize, sample_cnt: usize, sample_dims: usize, max_iter: usize, k: usize,
    ) where
        T: Primitive,
        LaneCount<LANES>: SupportedLaneCount,
        Simd<T, LANES>: SupportedSimdArray<T, LANES>,
    {
        let mut rnd = rand::rngs::StdRng::seed_from_u64(1337);
        let mut samples = vec![T::zero(); sample_cnt * sample_dims];
        samples.iter_mut().for_each(|v| *v = rnd.gen_range(T::zero()..T::one()));
        let kmean = KMeans::new(&samples, sample_cnt, sample_dims, EuclideanDistance);
        let conf = KMeansConfig::build().random_generator(rnd).build();
        b.iter(|| kmean.kmeans_minibatch(batch_size, k, max_iter, KMeans::init_random_sample, &conf));
    }
}
