//! Utilities to iterate over the `World` safely.

use ecs::bitset::BitSet;
use ecs::component::{Arena, Component};
use ecs::world::{Entities, EntitiesIter, Entity, World};

// use rayon::iter::plumbing::{bridge_unindexed, Folder, UnindexedConsumer, UnindexedProducer};
// use rayon::iter::ParallelIterator;

/// A arena with immutable read access into underlying components.
pub trait ArenaGet<T: Component> {
    /// Gets a reference to component `T`.
    fn get(&self, ent: Entity) -> Option<&T>;
    /// Gets a reference to component `T` without doing bounds checking.
    unsafe fn get_unchecked(&self, ent: Entity) -> &T;
}

/// A arena with mutable access into underlying components.
pub trait ArenaGetMut<T: Component>: ArenaGet<T> {
    /// Gets a mutable reference to component `T`.
    fn get_mut(&mut self, ent: Entity) -> Option<&mut T>;
    /// Gets a mutable reference to component `T` without doing bounds checking.
    unsafe fn get_unchecked_mut(&mut self, ent: Entity) -> &mut T;
}

pub struct Fetch<'w, T: Component> {
    arena: &'w T::Arena,
    world: &'w World,
}

impl<'w, T: Component> Fetch<'w, T> {
    pub(crate) unsafe fn new(world: &'w World) -> Self {
        Fetch {
            arena: world.arena::<T>(),
            world: world,
        }
    }
}

impl<'w, T: Component> ArenaGet<T> for Fetch<'w, T> {
    #[inline]
    fn get(&self, ent: Entity) -> Option<&T> {
        self.arena.get(ent.index())
    }

    #[inline]
    unsafe fn get_unchecked(&self, ent: Entity) -> &T {
        self.arena.get_unchecked(ent.index())
    }
}

pub struct FetchMut<'w, T: Component> {
    arena: &'w mut T::Arena,
    world: &'w World,
}

impl<'w, T: Component> FetchMut<'w, T> {
    pub(crate) unsafe fn new(world: &'w World) -> Self {
        FetchMut {
            arena: world.arena_mut::<T>(),
            world: world,
        }
    }
}

impl<'w, T: Component> ArenaGet<T> for FetchMut<'w, T> {
    #[inline]
    fn get(&self, ent: Entity) -> Option<&T> {
        self.arena.get(ent.index())
    }

    #[inline]
    unsafe fn get_unchecked(&self, ent: Entity) -> &T {
        self.arena.get_unchecked(ent.index())
    }
}

impl<'w, T: Component> ArenaGetMut<T> for FetchMut<'w, T> {
    #[inline]
    fn get_mut(&mut self, ent: Entity) -> Option<&mut T> {
        self.arena.get_mut(ent.index())
    }

    #[inline]
    unsafe fn get_unchecked_mut(&mut self, ent: Entity) -> &mut T {
        self.arena.get_unchecked_mut(ent.index())
    }
}

/// `Join` trait is used to provide a convenient way to access entities which
/// have specific components at the same time.
pub trait Join<'w>: Sized + 'w {
    type Item;

    /// Gets a iterator over entities and its specified components.
    #[inline]
    fn join(self) -> JoinIter<'w, Self> {
        unsafe {
            let mask = self.mask();
            let (v, world) = self.extract();
            JoinIter {
                iter: EntitiesIter::new(world, mask),
                values: v,
            }
        }
    }

    // /// Gets a parallel iterator over components with given step.
    // fn par_join<'w>(self, step: usize) -> ParJoinIter<'w, Self> {
    //     unsafe {
    //         assert!(step >= 1, "The divide step should always greater than 0.");

    //         let iter = EntitiesIter::new(self.world(), self.mask());
    //         ParJoinIter {
    //             iter: iter,
    //             values: self,
    //             step: step,
    //         }
    //     }
    // }

    #[doc(hidden)]
    unsafe fn extract(self) -> (Self, &'w World);
    #[doc(hidden)]
    unsafe fn mask(&self) -> BitSet;
    #[doc(hidden)]
    unsafe fn get_unchecked(values: &Self, id: Entity) -> Self::Item;
}

/// The `JoinIter` iterates over a group of entities which have associated
/// `Component`s, and returns the corresponding items.
pub struct JoinIter<'w, J: Join<'w> + 'w> {
    iter: EntitiesIter<'w>,
    values: J,
}

impl<'w, J: Join<'w> + 'w> Iterator for JoinIter<'w, J> {
    type Item = J::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|id| unsafe { J::get_unchecked(&self.values, id) })
    }
}

impl<'w> Join<'w> for Entities<'w> {
    type Item = Entity;

    #[inline]
    unsafe fn extract(self) -> (Self, &'w World) {
        (self, self.world)
    }

    #[inline]
    unsafe fn mask(&self) -> BitSet {
        BitSet::new()
    }

    #[inline]
    unsafe fn get_unchecked(_: &Self, id: Entity) -> Self::Item {
        id
    }
}

impl<'r, 'w: 'r> Join<'r> for &'r Entities<'w> {
    type Item = Entity;

    #[inline]
    unsafe fn extract(self) -> (Self, &'r World) {
        (self, self.world)
    }

    #[inline]
    unsafe fn mask(&self) -> BitSet {
        BitSet::new()
    }

    #[inline]
    unsafe fn get_unchecked(_: &Self, id: Entity) -> Self::Item {
        id
    }
}

impl<'r, 'w: 'r> Join<'r> for &'r mut Entities<'w> {
    type Item = Entity;

    #[inline]
    unsafe fn extract(self) -> (Self, &'r World) {
        (self, self.world)
    }

    #[inline]
    unsafe fn mask(&self) -> BitSet {
        BitSet::new()
    }

    #[inline]
    unsafe fn get_unchecked(_: &Self, id: Entity) -> Self::Item {
        id
    }
}

impl<'w, C: Component> Join<'w> for Fetch<'w, C> {
    type Item = &'w C;

    #[inline]
    unsafe fn extract(self) -> (Self, &'w World) {
        let w = self.world;
        (self, w)
    }

    #[inline]
    unsafe fn mask(&self) -> BitSet {
        BitSet::from(&[self.world.mask_index::<C>()])
    }

    #[inline]
    unsafe fn get_unchecked(values: &Self, id: Entity) -> Self::Item {
        (&*(values as *const Self)).get_unchecked(id)
    }
}

impl<'r, 'w: 'r, C: Component> Join<'r> for &'r Fetch<'w, C> {
    type Item = &'r C;

    #[inline]
    unsafe fn extract(self) -> (Self, &'r World) {
        (self, self.world)
    }

    #[inline]
    unsafe fn mask(&self) -> BitSet {
        BitSet::from(&[self.world.mask_index::<C>()])
    }

    #[inline]
    unsafe fn get_unchecked(values: &Self, id: Entity) -> Self::Item {
        values.get_unchecked(id)
    }
}

impl<'w, C: Component> Join<'w> for FetchMut<'w, C> {
    type Item = &'w mut C;

    #[inline]
    unsafe fn extract(self) -> (Self, &'w World) {
        let w = self.world;
        (self, w)
    }

    #[inline]
    unsafe fn mask(&self) -> BitSet {
        BitSet::from(&[self.world.mask_index::<C>()])
    }

    #[inline]
    unsafe fn get_unchecked(values: &Self, id: Entity) -> Self::Item {
        (&mut *(values as *const Self as *mut Self)).get_unchecked_mut(id)
    }
}

impl<'r, 'w: 'r, C: Component> Join<'r> for &'r FetchMut<'w, C> {
    type Item = &'r C;

    #[inline]
    unsafe fn extract(self) -> (Self, &'r World) {
        (self, self.world)
    }

    #[inline]
    unsafe fn mask(&self) -> BitSet {
        BitSet::from(&[self.world.mask_index::<C>()])
    }

    #[inline]
    unsafe fn get_unchecked(values: &Self, id: Entity) -> Self::Item {
        values.get_unchecked(id)
    }
}

impl<'r, 'w: 'r, C: Component> Join<'r> for &'r mut FetchMut<'w, C> {
    type Item = &'r mut C;

    #[inline]
    unsafe fn extract(self) -> (Self, &'r World) {
        (self, self.world)
    }

    #[inline]
    unsafe fn mask(&self) -> BitSet {
        BitSet::from(&[self.world.mask_index::<C>()])
    }

    #[inline]
    unsafe fn get_unchecked(values: &Self, id: Entity) -> Self::Item {
        (&mut *(values as *const Self as *mut Self)).get_unchecked_mut(id)
    }
}

macro_rules! impl_join {
    ([$head: ident, $($tails: ident), *]) => (
        impl<'w, $head: Join<'w>, $($tails: Join<'w>, )*> Join<'w> for ( $head, $($tails,)* ) {
            type Item = ( $head::Item, $($tails::Item, ) * );

            #[inline]
            #[allow(non_snake_case)]
            unsafe fn extract(self) -> (Self, &'w World) {
                let ($head, $($tails, )*) = self;
                let ($head, world) = $head.extract();
                (( $head, $($tails, )* ), world)
            }

            #[inline]
            #[allow(non_snake_case)]
            unsafe fn mask(&self) -> BitSet {
                let &(ref $head, $(ref $tails, )*) = self;
                let mut mask = BitSet::new();
                mask = mask.union_with($head.mask());
                $( mask = mask.union_with($tails.mask()); ) *
                mask
            }

            #[inline]
            #[allow(non_snake_case)]
            unsafe fn get_unchecked(values: &Self, id: Entity) -> Self::Item {
                let &(ref $head, $(ref $tails, )*) = values;
                ( $head::get_unchecked(&$head, id), $($tails::get_unchecked(&$tails, id), )* )
            }
        }
    );
}

impl_join!([T1, T2]);
impl_join!([T1, T2, T3]);
impl_join!([T1, T2, T3, T4]);
impl_join!([T1, T2, T3, T4, T5]);
impl_join!([T1, T2, T3, T4, T5, T6]);
impl_join!([T1, T2, T3, T4, T5, T6, T7]);
impl_join!([T1, T2, T3, T4, T5, T6, T7, T8]);
impl_join!([T1, T2, T3, T4, T5, T6, T7, T8, T9]);

// /// The parallel `JoinIter` based on rayon facilities.
// pub struct ParJoinIter<'w, J: Join + 'w> {
//     iter: EntitiesIter<'w>,
//     values: J,
//     step: usize,
// }

// impl<'w, J: Join> ParallelIterator for ParJoinIter<'w, J>
// where
//     J: Join + Send,
//     J::Item: Send,
// {
//     type Item = J::Item;

//     fn drive_unindexed<C>(self, consumer: C) -> C::Result
//     where
//         C: UnindexedConsumer<Self::Item>,
//     {
//         let values = UnsafeCell::new(self.values);
//         let producer = ParJoinProducer::new(&values, self.iter, self.step);
//         bridge_unindexed(producer, consumer)
//     }
// }

// struct ParJoinProducer<'a, 'w, J: Join + 'a> {
//     iter: EntitiesIter<'w>,
//     values: &'a UnsafeCell<J>,
//     step: usize,
// }

// impl<'a, 'w, J: Join + 'a> ParJoinProducer<'a, 'w, J> {
//     fn new(values: &'a UnsafeCell<J>, iter: EntitiesIter<'w>, step: usize) -> Self {
//         ParJoinProducer {
//             iter: iter,
//             values: values,
//             step: step,
//         }
//     }
// }

// unsafe impl<'a, 'w, J: Join + 'a> Send for ParJoinProducer<'a, 'w, J> {}

// impl<'a, 'w, J: Join + 'a> UnindexedProducer for ParJoinProducer<'a, 'w, J> {
//     type Item = J::Item;

//     fn split(self) -> (Self, Option<Self>) {
//         if self.iter.len() <= self.step {
//             (self, None)
//         } else {
//             let (left, right) = self.iter.split();
//             let values = self.values;

//             (
//                 ParJoinProducer::new(values, left, self.step),
//                 Some(ParJoinProducer::new(values, right, self.step)),
//             )
//         }
//     }

//     fn fold_with<F>(self, folder: F) -> F
//     where
//         F: Folder<Self::Item>,
//     {
//         let ParJoinProducer { values, iter, .. } = self;
//         let iter = iter.map(|id| unsafe { J::get_unchecked(&mut *values.get(), id) });
//         folder.consume_iter(iter)
//     }
// }
