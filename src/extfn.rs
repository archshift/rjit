use crate::mem;

pub struct ExtFn {
    pub(crate) ptr: usize
}

impl ExtFn {
    pub fn f(&self) -> extern fn() {
        unsafe { mem::transmute(self.ptr) }
    } 
    pub unsafe fn fn0<Res>(&self) -> extern fn() -> Res {
        mem::transmute(self.ptr)
    }
    pub unsafe fn fn1<A, Res>(&self) -> extern fn(A) -> Res {
        mem::transmute(self.ptr)
    }
    pub unsafe fn fn2<A, B, Res>(&self) -> extern fn(A, B) -> Res {
        mem::transmute(self.ptr)
    }
    pub unsafe fn fn3<A, B, C, Res>(&self) -> extern fn(A, B, C) -> Res {
        mem::transmute(self.ptr)
    }
    pub unsafe fn fn4<A, B, C, D, Res>(&self) -> extern fn(A, B, C, D) -> Res {
        mem::transmute(self.ptr)
    }
    pub unsafe fn fn5<A, B, C, D, E, Res>(&self) -> extern fn(A, B, C, D, E) -> Res {
        mem::transmute(self.ptr)
    }
    pub unsafe fn fn_var<A, Res>(&self) -> extern fn(A, ...) -> Res {
        mem::transmute(self.ptr)
    }
}
