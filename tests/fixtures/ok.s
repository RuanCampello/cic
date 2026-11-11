  #
  # modelo de saida para o compilador
  #

  .section .text
  .globl _start

_start:
  mov $5, %rax
  push %rax
  mov $231, %rax
  pop %rbx
  add %rbx, %rax
  push %rax
  mov $11, %rax
  pop %rbx
  imul %rbx, %rax
  push %rax
  mov $7, %rax
  push %rax
  mov $427, %rax
  pop %rbx
  cqo
  idiv %rbx
  pop %rbx
  add %rbx, %rax

  call imprime_num
  call sair

  .include "runtime.s"
