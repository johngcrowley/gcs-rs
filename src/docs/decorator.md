Decorator _is_ wrapping and wrapping types in new types, but giving you a single interface method
whereby you call it on the outside and it cascades down to the innermost's call.

A canvas > Scrollwindow > Resizable Window > Frame > 

the user doesn't need to know how many layers of indirection there are. 

you just have "height" and "width" get call which cascades down from Frame to Canvas.

---

Method-chaining _is_ tough because you need 47 tabs open to check return types of 
each modification to the original type.

---
