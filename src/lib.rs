use std::{ops::Index, ptr};

const ISIZE_MAX_SIZE: usize = isize::MAX as usize;

pub struct LeVec<T> {
    pub ptr: ptr::NonNull<T>,
    pub len: usize,
    pub cap: usize,
}

impl<T> LeVec<T> {
    pub fn new() -> Self {
        Self {
            ptr: ptr::NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn push(&mut self, value: T) {
        let size = std::mem::size_of::<T>();
        //TODO: see what std does when size == 0
        assert!(size > 0, "size of T must be greater than 0");
        if self.len == 0 {
            let new_size = size.checked_mul(4).expect("capacity overflow");
            assert!(new_size <= ISIZE_MAX_SIZE, "capacity overflow");
            let layout = std::alloc::Layout::array::<T>(4).unwrap();

            //SAFETY: layout is size_of::<T>() * 4 and size_of::<T>() > 0
            let ptr = unsafe { std::alloc::alloc(layout) as *mut T };

            let ptr = ptr::NonNull::new(ptr).expect("allocation failed");

            //SAFETY: ptr is non-null,the value is not read and the value is not dropped
            unsafe { ptr.as_ptr().write(value) };
            self.ptr = ptr;
            self.len = 1;
            self.cap = 4;
        } else if self.len < self.cap {
            let offset = self.len.checked_mul(size).expect("capacity overflow");
            assert!(offset <= isize::MAX as usize, "capacity overflow");

            //SAFETY: offset is less than capacity, offset fits in isize
            unsafe {
                self.ptr.as_ptr().add(self.len).write(value);
            }
            self.len += 1;
        } else {
            let new_cap = self.cap.checked_mul(2).expect("capacity overflow");
            let new_size = size.checked_mul(new_cap).expect("capacity overflow");
            assert!(new_size <= isize::MAX as usize, "capacity overflow");
            let layout = std::alloc::Layout::array::<T>(new_cap).unwrap();

            // Calculate the maximum size that can be represented by isize_max_size
            // when rounded up to the nearest multiple of layout.align()
            let aligned_isize_max_size = ISIZE_MAX_SIZE + (layout.align() - 1) as usize;
            let aligned_isize_max_size_rounded =
                aligned_isize_max_size - (aligned_isize_max_size % layout.align());

            assert!(
                new_size <= aligned_isize_max_size_rounded,
                "capacity overflow"
            );

            /*SAFETY:
                ptr is non-null
                ptr was allocated via this allocator
                layout is the same layout that was used to allocate ptr
                new_size when rounded up to the nearest multiple of layout.align() fits in isize
            */
            let ptr = unsafe {
                std::alloc::realloc(self.ptr.as_ptr() as *mut u8, layout, new_size) as *mut T
            };
            let ptr = ptr::NonNull::new(ptr).expect("allocation failed");
            unsafe {
                ptr.as_ptr().add(self.len).write(value);
            }
            self.ptr = ptr;
            self.len += 1;
            self.cap = new_cap;
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            //SAFETY: index is less than length
            unsafe { Some(&*self.ptr.as_ptr().add(index)) }
        } else {
            None
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            //SAFETY: self.len is greater than 0
            unsafe { Some(self.ptr.as_ptr().add(self.len).read()) }
        } else {
            None
        }
    }
}

impl<T> Index<usize> for LeVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}

impl<T> Drop for LeVec<T> {
    fn drop(&mut self) {
        for i in 0..self.len {
            //SAFETY: i is less than length
            unsafe {
                self.ptr.as_ptr().add(i).drop_in_place();
            }
        }

        let layout = std::alloc::Layout::array::<T>(self.cap).unwrap();
        /*
           SAFETY:
               ptr is non-null
               ptr was allocated via this allocator
               layout is the same layout that was used to allocate ptr
        */
        unsafe { std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout) };
    }
}

impl<T> Iterator for LeVec<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len > 0 {
            self.len -= 1;
            //SAFETY: self.len is greater than 0
            unsafe { Some(self.ptr.as_ptr().add(self.len).read()) }
        } else {
            None
        }
    }
}

impl<'a, T> IntoIterator for &'a LeVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        //SAFETY: self.ptr is non-null
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len).iter() }
    }
}
#[cfg(test)]
mod test {

    #[derive(Debug, PartialEq)]
    struct Dropped(String);

    impl Drop for Dropped {
        fn drop(&mut self) {
            println!("Dropped {}", self.0);
        }
    }

    use super::*;

    #[test]
    fn test_push() {
        let mut vec = LeVec::new();
        vec.push(Dropped("1".to_string()));
        vec.push(Dropped("2".to_string()));
        vec.push(Dropped("3".to_string()));
        vec.push(Dropped("4".to_string()));
        vec.push(Dropped("5".to_string()));
        vec.push(Dropped("6".to_string()));
        vec.push(Dropped("7".to_string()));

        assert_eq!(vec.len(), 7);
        assert_eq!(vec.capacity(), 8);

        vec.pop();
        vec.pop();

        assert_eq!(vec.len(), 5);
        assert_eq!(vec.capacity(), 8);

        for value in &vec {
            println!("&iter {:?}", value);
        }

        for value in vec {
            println!("iter {:?}", value);
        }
    }
}
