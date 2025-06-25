use serde::{
    de::{DeserializeSeed, IgnoredAny, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

use crate::{paginator::Paginator, types::Step};

#[derive(Debug)]
pub struct PaginatedVec<T> {
    data: Vec<T>,
    paginator: Paginator,
}

impl<T> PaginatedVec<T> {
    pub fn new(paginator: Paginator) -> Self {
        let data = Vec::default();
        Self { data, paginator }
    }

    pub fn data(&self) -> &Vec<T> {
        &self.data
    }

    pub fn paginator(&self) -> &Paginator {
        &self.paginator
    }
}

pub struct PaginatedVecWrapper<'a, T: 'a>(pub &'a mut PaginatedVec<T>, pub &'a mut Step);

impl<'de, 'a, T> DeserializeSeed<'de> for PaginatedVecWrapper<'a, T>
where
    T: Deserialize<'de>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PaginatedVecWrapperVisitor<'a, T: 'a>(&'a mut PaginatedVec<T>, pub &'a mut Step);

        impl<'de, 'a, T> Visitor<'de> for PaginatedVecWrapperVisitor<'a, T>
        where
            T: Deserialize<'de>,
        {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "an array of integers")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut current_start = 0;
                let mut total_counter = 0;

                if let Some(total) = self.0.paginator.total() {
                    loop {
                        if *self.1 == Step::MAX {
                            if total < self.0.paginator.size() {
                                break;
                            } else if current_start >= total - self.0.paginator.size() {
                                break;
                            }
                        }

                        if current_start <= *self.1
                            && *self.1 < current_start + self.0.paginator.size()
                        {
                            break;
                        }

                        for _ in 0..*self.0.paginator.size() {
                            seq.next_element::<IgnoredAny>()?;
                        }

                        current_start += self.0.paginator.size();
                    }
                }

                loop {
                    let mut sliding_vec: Vec<T> = Vec::with_capacity(*self.0.paginator.size());

                    for _ in 0..*self.0.paginator.size() {
                        if let Some(value) = seq.next_element()? {
                            sliding_vec.push(value);
                            total_counter += 1;
                        } else {
                            break;
                        }
                    }

                    if current_start <= *self.1 && *self.1 < current_start + self.0.paginator.size()
                    {
                        self.0.paginator.start_update(current_start);
                        self.0.data = sliding_vec;
                        break;
                    }

                    if sliding_vec.is_empty() {
                        current_start -= self.0.paginator.size();
                        self.0.paginator.start_update(current_start);
                        *self.1 = current_start + self.0.paginator.size() - 1;
                        break;
                    } else if sliding_vec.len() < *self.0.paginator.size() {
                        self.0.data = sliding_vec;
                        self.0.paginator.start_update(current_start);
                        *self.1 = current_start + self.0.data.len() - 1;
                        break;
                    } else {
                        self.0.data = sliding_vec;
                    }

                    current_start += self.0.paginator.size();
                }

                while let Some(IgnoredAny) = seq.next_element()? {
                    total_counter += 1;
                }

                // FIXME Dirty hack
                if self.0.data.len() <= *self.1 && *self.0.paginator.start() == 0 {
                    *self.1 = (*self.1).min(Step::from(total_counter).saturating_sub(1));
                }

                if self.0.paginator.total().is_none() {
                    self.0.paginator.total_update(Some(total_counter));
                }

                Ok(())
            }
        }

        deserializer.deserialize_seq(PaginatedVecWrapperVisitor(self.0, self.1))
    }
}
