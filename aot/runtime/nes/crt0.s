; aot/runtime/nes/crt0.s — NES startup, NMI, and UNROM bank switching for the
; PocketJS-AOT runtime (ca65 syntax, linked by the generated nes.cfg).
;
; The NMI handler owns all PPU traffic while rendering is on:
;   1. OAM DMA from the $0200 shadow page
;   2. flush of the VRAM update buffer (_pj_vbuf): entries [hi,lo,len,data...]
;      terminated by hi = $FF. Main-thread code only appends to the buffer
;      (RAM), so the NMI never depends on the current ROM bank.
;   3. scroll + PPUCTRL restore, frame flag increment.
; When _pj_ppu_off is nonzero (map reloads), the NMI only bumps the flag.

.export __STARTUP__ : absolute = 1
.export _pj_bank_switch
.export _exit
.import _main, zerobss, copydata
.import _pj_vbuf, _pj_nmi_flag, _pj_ppu_off, _pj_ppuctrl
.importzp sp

PPUCTRL   = $2000
PPUMASK   = $2001
PPUSTATUS = $2002
PPUSCROLL = $2005
OAMDMA    = $4014

.segment "STARTUP"

reset:
    sei
    cld
    ldx #$40
    stx $4017            ; APU frame IRQ off
    ldx #$FF
    txs
    inx                  ; x = 0
    stx PPUCTRL          ; NMI off
    stx PPUMASK          ; rendering off
    stx $4010            ; DMC IRQ off
    bit PPUSTATUS

@vbl1:
    bit PPUSTATUS
    bpl @vbl1

    ; clear all 2 KB of RAM (incl. shadow OAM + debug page)
    txa
@clear:
    sta $0000,x
    sta $0100,x
    sta $0200,x
    sta $0300,x
    sta $0400,x
    sta $0500,x
    sta $0600,x
    sta $0700,x
    inx
    bne @clear

    ; park sprites offscreen (y = $FF)
    lda #$FF
@oam:
    sta $0200,x
    inx
    inx
    inx
    inx
    bne @oam

@vbl2:
    bit PPUSTATUS
    bpl @vbl2

    lda #0
    jsr _pj_bank_switch  ; map bank 0 into $8000

    jsr zerobss
    jsr copydata

    ; cc65 software stack: top at $0700 (the debug page sits above)
    lda #$00
    sta sp
    lda #$07
    sta sp+1

    jsr _main
_exit:
    jmp _exit

; ------------------------------------------------------------------ NMI ----
.segment "CODE"

nmi:
    pha
    txa
    pha
    tya
    pha

    lda _pj_ppu_off
    bne @tick            ; rendering off: main thread owns the PPU

    lda #$02
    sta OAMDMA

    ; flush the VRAM update buffer
    ldx #0
@entry:
    lda _pj_vbuf,x
    cmp #$FF
    beq @flushed
    sta PPUADDR_HI
    inx
    lda _pj_vbuf,x
    sta PPUADDR_LO
    inx
    lda _pj_vbuf,x       ; len
    tay
    inx
@data:
    lda _pj_vbuf,x
    sta $2007
    inx
    dey
    bne @data
    jmp @entry
@flushed:
    lda #$FF
    sta _pj_vbuf

    ; reset scroll (no scrolling on NES v1) + restore ctrl
    bit PPUSTATUS
    lda #0
    sta PPUSCROLL
    sta PPUSCROLL
    lda _pj_ppuctrl
    sta PPUCTRL

@tick:
    inc _pj_nmi_flag

    pla
    tay
    pla
    tax
    pla
irq:
    rti

; PPUADDR needs hi then lo; give the two stores distinct names for clarity.
PPUADDR_HI = $2006
PPUADDR_LO = $2006

; ------------------------------------------- UNROM bank switch (fastcall a=bank)
_pj_bank_switch:
    tax
    sta banktable,x      ; write value == ROM content: no bus conflict
    rts

.segment "RODATA"
banktable:
    .byte 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15

.segment "VECTORS"
    .word nmi
    .word reset
    .word irq
