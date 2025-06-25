use indexmap::IndexMap;
use serde::{
    de::{DeserializeSeed, IgnoredAny, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

use crate::{paginator::Paginator, types::Step};

#[derive(Debug)]
pub struct PaginatedMap<K, V> {
    data: IndexMap<K, V>,
    paginator: Paginator,
}

impl<K, V> PaginatedMap<K, V> {
    pub fn new(paginator: Paginator) -> Self {
        let data = IndexMap::new();

        Self { data, paginator }
    }

    pub fn data(&self) -> &IndexMap<K, V> {
        &self.data
    }

    pub fn paginator(&self) -> &Paginator {
        &self.paginator
    }
}

pub struct PaginatedMapWrapper<'a, K: 'a, V: 'a>(pub &'a mut PaginatedMap<K, V>, pub &'a mut Step);

impl<'de, 'a, K, V> DeserializeSeed<'de> for PaginatedMapWrapper<'a, K, V>
where
    K: Deserialize<'de> + std::cmp::Eq + std::hash::Hash + std::cmp::Ord,
    V: Deserialize<'de>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PaginatedMapWrapperVisitor<'a, K: 'a, V: 'a>(
            &'a mut PaginatedMap<K, V>,
            pub &'a mut Step,
        );

        impl<'de, 'a, K, V> Visitor<'de> for PaginatedMapWrapperVisitor<'a, K, V>
        where
            K: Deserialize<'de> + std::cmp::Eq + std::hash::Hash + std::cmp::Ord,
            V: Deserialize<'de>,
        {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a map")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // return Err(serde::de::Error::custom("my custom error"));

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
                            map.next_entry::<IgnoredAny, IgnoredAny>()?;
                        }

                        current_start += self.0.paginator.size();
                    }
                }

                loop {
                    let mut sliding_map: IndexMap<K, V> =
                        IndexMap::with_capacity(*self.0.paginator.size());

                    for _ in 0..*self.0.paginator.size() {
                        if let Some((key, value)) = map.next_entry::<K, V>()? {
                            sliding_map.insert(key, value);
                            total_counter += 1;
                        } else {
                            break;
                        }
                    }

                    if current_start <= *self.1 && *self.1 < current_start + self.0.paginator.size()
                    {
                        self.0.paginator.start_update(current_start);
                        self.0.data = sliding_map;
                        break;
                    }

                    if sliding_map.is_empty() {
                        current_start -= self.0.paginator.size();
                        self.0.paginator.start_update(current_start);
                        *self.1 = current_start + self.0.paginator.size() - 1;
                        break;
                    } else if sliding_map.len() < *self.0.paginator.size() {
                        self.0.data = sliding_map;
                        self.0.paginator.start_update(current_start);
                        *self.1 = current_start + self.0.data.len() - 1;
                        break;
                    } else {
                        self.0.data = sliding_map;
                    }

                    current_start += self.0.paginator.size();
                }

                while let Some((IgnoredAny, IgnoredAny)) = map.next_entry()? {
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

        deserializer.deserialize_map(PaginatedMapWrapperVisitor(self.0, self.1))
    }
}
