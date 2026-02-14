# Connect to QEMU and load symbols
target remote localhost:1234
symbol-file target/aarch64-unknown-none-softfloat/debug/l4sm

# Disassemble N instructions from the current PC (default: 10)
define asm
  if $argc == 0
    x/10i $pc
  else
    x/$arg0i $pc
  end
end
document asm
Usage: asm [N]
Disassemble the next N instructions starting from the current program counter.
If N is omitted, defaults to 10.
end

break _start
c
