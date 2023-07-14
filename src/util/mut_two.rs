// Takes 2 indexes of an array and returns the mutable item corresponding to both
// This is done by splitting the array into 2 mutable chunks, which contain the
// two desired items. The items are then indexed from the 2 slices, and then returned
pub fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
    assert!(first_index != second_index);
    if first_index < second_index {
        let (first_slice, second_slice) = items.split_at_mut(second_index);
        (&mut first_slice[first_index], &mut second_slice[0])
    } else {
        let (first_slice, second_slice) = items.split_at_mut(first_index);
        (&mut second_slice[0], & mut first_slice[second_index])
    }
}
