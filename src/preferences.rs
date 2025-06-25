use impl_helper::ImplHelper;
use wrap_context::{arg_context, liab};

pub const MIN_LEFT_TABLE_COLUMN_WIDTH: u16 = 15;
pub const MIN_RGHT_TABLE_COLUMN_WIDTH: u16 = 15;

#[derive(Debug, ImplHelper)]
pub struct Preferences {
    #[helper(get /* upd is custom */)]
    left_table_column_width: u16,

    #[helper(get /* upd is custom */)]
    rght_table_column_width: u16,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            left_table_column_width: MIN_LEFT_TABLE_COLUMN_WIDTH,
            rght_table_column_width: MIN_RGHT_TABLE_COLUMN_WIDTH,
        }
    }
}

impl Preferences {
    pub fn apply_term_width(&mut self, term_width: u16) -> anyhow::Result<[Option<u16>; 2]> {
        let left_width = self.left_table_column_width;
        let rght_width = self.rght_table_column_width;

        let mut new_left_width = None;
        let mut new_rght_width = None;

        if (left_width + rght_width) > term_width {
            let temp_left_width = term_width
                .checked_sub(rght_width)
                .unwrap_or(MIN_LEFT_TABLE_COLUMN_WIDTH)
                .clamp(MIN_LEFT_TABLE_COLUMN_WIDTH, u16::MAX);
            arg_context!(self.left_table_column_width_update(temp_left_width))?;
            new_left_width = Some(temp_left_width);

            if (temp_left_width + rght_width) > term_width {
                let temp_right_width = term_width
                    .checked_sub(temp_left_width)
                    .unwrap_or(MIN_RGHT_TABLE_COLUMN_WIDTH)
                    .clamp(MIN_RGHT_TABLE_COLUMN_WIDTH, u16::MAX);
                arg_context!(self.rght_table_column_width_update(temp_right_width))?;
                new_rght_width = Some(temp_right_width);
            }
        }

        anyhow::Ok([new_left_width, new_rght_width])
    }

    pub fn left_table_column_width_update(&mut self, value: u16) -> anyhow::Result<&mut Self> {
        if value < MIN_LEFT_TABLE_COLUMN_WIDTH {
            liab!("Not enough space for left table!");
        }

        self.left_table_column_width = value;
        anyhow::Ok(self)
    }

    pub fn left_table_column_width_inc(&mut self) -> Option<u16> {
        let result = self.left_table_column_width.checked_add(1);
        if let Some(new_width) = result {
            self.left_table_column_width = new_width;
        }
        result
    }

    pub fn left_table_column_width_dec(&mut self) -> Option<u16> {
        if self.left_table_column_width > MIN_LEFT_TABLE_COLUMN_WIDTH {
            let result = self.left_table_column_width.checked_sub(1);
            if let Some(new_width) = result {
                self.left_table_column_width = new_width;
            }
            result
        } else {
            None
        }
    }

    pub fn rght_table_column_width_update(&mut self, value: u16) -> anyhow::Result<&mut Self> {
        if value < MIN_RGHT_TABLE_COLUMN_WIDTH {
            liab!("Not enough space for right table!");
        }

        self.rght_table_column_width = value;
        anyhow::Ok(self)
    }

    pub fn rght_table_column_width_inc(&mut self) -> Option<u16> {
        let result = self.rght_table_column_width.checked_add(1);
        if let Some(new_width) = result {
            self.rght_table_column_width = new_width;
        }
        result
    }

    pub fn rght_table_column_width_dec(&mut self) -> Option<u16> {
        if self.rght_table_column_width > MIN_RGHT_TABLE_COLUMN_WIDTH {
            let result = self.rght_table_column_width.checked_sub(1);
            if let Some(new_width) = result {
                self.rght_table_column_width = new_width;
            }
            result
        } else {
            None
        }
    }
}
