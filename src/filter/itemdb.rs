use {mu, ext};

#[derive(Debug, Clone)]
pub struct ItemDb {
  pub name: String,
  pub code: mu::ItemCode,
  pub level: Option<u8>,
}

lazy_static! {
  /// A collection of meta information for buffs.
  pub static ref ITEMDB: Vec<ItemDb> = {
    let mut result = Vec::with_capacity(2000);
    let mut temp: Vec<ItemDb> = Vec::with_capacity(15);

    for group in 0..16 {
      'id: for id in 0..200 {
        let code = mu::ItemCode::from_code(group * 512 + id);

        for level in 0..16 {
          let name = unsafe { ext::func::get_item_name(code, level) };

          if name.is_empty() {
            if level == 0 {
              // There is most likely no item with this code
              break;
            } else {
              // There are often gaps between the item levels
              continue;
            }
          }

          // Determine if the name is merely part of the previous
          if name.contains("+") { break; }

          // This item level may be a duplicate of the previous one
          if temp.last().map(|item| item.name == name).unwrap_or(false) { break; }

          temp.push(ItemDb { name, code, level: Some(level) });
        }

        if temp.len() == 1 {
          // The level is not used if the code is unique
          temp[0].level = None;
        }

        result.extend_from_slice(&temp);
        temp.clear();
      }
    }

    result
  };
}