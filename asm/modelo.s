  #
  # modelo de saida para o compilador
  #

  .section .bss
  ## variaveis devem ser declaradas aqui

  .section .text
  .globl _start

_start:
  ## saida do compilador deve ser inserida aqui

  call imprime_num
  call sair

  .include "runtime.s"
