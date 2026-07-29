; boot/bootloader.asm
BITS 64
global _start

_start:
    ; Carregar kernel ELF do IPFS (CID fixo no setor 1)
    ; Verificar assinatura Ed25519 (syscall 0x989)
    ; Configurar páginação (4-level paging)
    ; Saltar para o entry point do kernel
    jmp kmain

kmain:
    hlt
