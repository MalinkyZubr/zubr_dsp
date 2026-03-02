use num::{Num, Signed, ToPrimitive};

pub fn is_power_2(num: i32) -> bool {
    return ((num - 1) & num) == 0;
}

pub fn percent_error<T: Num + Signed + Clone + ToPrimitive + Into<f32>>(
    observed: T,
    expected: T,
) -> f32 {
    let expected_clone = expected.clone();
    return ((observed.into() - (expected.into())) / expected_clone.into()).abs() * 100.0;
}

pub enum BoundType {
    Inclusive,
    Exclusive,
}

pub fn is_within_bounds<T: Num + Signed + PartialOrd>(
    term: T,
    bounds: (T, T),
    bound_types: (BoundType, BoundType),
) -> bool {
    let obey_lower_bound = match bound_types.0 {
        BoundType::Inclusive => term >= bounds.0,
        BoundType::Exclusive => term > bounds.0,
    };
    let obey_upper_bound = match bound_types.1 {
        BoundType::Inclusive => term <= bounds.1,
        BoundType::Exclusive => term < bounds.1,
    };

    return obey_lower_bound && obey_upper_bound;
}
