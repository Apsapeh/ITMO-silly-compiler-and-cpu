## Types:
 + `u32/i32` - 32 bit integer (machine word)
 + `u64/i64` - 64 bit integer



## Pointers:
  `& <var>`  - Ref data, like in C, not pointers
  `* <var>`  - Deref pointer


## Strings:
  `&u32` | `cstr` - null-terminated string (each char is UTF-32)
  

```lua
function fib(n: u32): u32
  if n < 2 then
    return 1
  end
  ...
end
```


```rust
  // Comment

  fn fib(n: u32) -> u32 {
      if n < 2 { return 1; }
      return fib(n - 2) + fib(n - 1);
  }

  fn strlen(string: &u32) -> u32 {
      let len: u32 = 0;
      while *string {
        string += 1;
      }
      return len;
  }


  // Entry point
  fn main() {
      // Branches
      let number: u32 = 10;
      if number == 50 {
        print("If");
      } else if number == 10 {
        print("Elseif");
      } else {
        print("Else");
      }

      // Loops
      let counter: u32 = 10;
      while counter {
          print("");
          counter -= 1;
      }
  
      let n: u32 = 10;
      print(fib(n));
  }
```


=
==

+
+=

-
-=

*
*=

/
/=

%
%=

!
!=

~
~=

<!-- ^ -->
<!-- ^= -->

>
>=
>>
>>=

<
<=
<<
<<=

&
&=
&&
<!-- &&= -->

|
|=
||
<!-- ||= -->
