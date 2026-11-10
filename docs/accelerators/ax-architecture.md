# APOLO AX — arquitetura própria de aceleradores

## Objetivo

Desenvolver uma arquitetura de aceleradores APOLO, incluindo ISA, formato de programa, compilador, runtime, driver e implementação lógica/hardware. O AX Accelerator crate é a primeira fundação de software: define um formato de comandos versionado e uma fronteira de backend independente do fabricante.

## ISA preliminar v0.1

A versão inicial reserva operações para:
- NOP e barreira de sincronização.
- LOAD e STORE com verificação de faixa de memória.
- ADD e MULTIPLY.
- FUSED MULTIPLY-ADD.
- DOT PRODUCT.
- RELU.
- MATRIX MULTIPLY TILE.

Cada programa declara versão ISA, quantidade de registradores e tamanho da região de memória de dispositivo. A validação rejeita registradores fora da faixa, operandos incompatíveis, versão não suportada e acesso de memória fora dos limites.

## Contrato de dispositivo

O descritor identifica recurso, nó, arquitetura, versão ISA, registradores e memória. A submissão exige dispositivo saudável e hardware verificado. O agente do nó não pode atribuir essa confiança por autorrelato.

## O que ainda não existe

- Não há silício, placa, FPGA bitstream, RTL sintetizável ou driver de hardware nesta etapa.
- Não há benchmark, FLOPS medidos, largura de banda medida ou comparação de desempenho.
- O trait `AcceleratorBackend` é apenas a fronteira para implementação futura de drivers.
- O ISA é preliminar e poderá mudar antes de congelamento de ABI.

## Roteiro de hardware

1. Especificar formato binário, convenção de registradores, memória, interrupções e modelo de execução.
2. Criar referência funcional independente e suíte de conformidade.
3. Escrever RTL SystemVerilog para pipeline escalar/vectorial e unidades matriciais.
4. Validar RTL com simulação, assertions e testes de equivalência.
5. Sintetizar em FPGA disponível e medir frequência, latência, consumo e throughput.
6. Desenvolver driver Linux e runtime APOLO.
7. Só então avaliar ASIC, fabricação, encapsulamento, alimentação e validação elétrica.

O software pode ser desenvolvido sem placa, mas execução física e desempenho só serão demonstrados em hardware real.

## AX Reference Runtime — primeiro incremento

O módulo `reference_runtime` interpreta programas AX em CPU comum para validar semântica funcional. Ele não simula frequência, paralelismo, latência ou desempenho de um acelerador físico.

Implementado neste incremento:
- Registradores de 64 bits inicializados em zero.
- LOAD/STORE little-endian para larguras de 1, 2, 4 ou 8 bytes.
- ADD, MULTIPLY, FUSED MULTIPLY-ADD e RELU com semântica inteira definida; operações aritméticas usam wrap modulo 2^64.
- DOT PRODUCT, nesta versão escalar, calcula o produto de dois valores; produto vetorial/acumulado ainda requer definição formal.
- NOP e BARRIER são operações sem efeito no interpretador sequencial.
- Limite de instruções por execução e verificação de que a região declarada cabe no buffer fornecido.
- MATRIX MULTIPLY TILE é rejeitado explicitamente como não implementado, em vez de produzir resultado fictício.

O runtime é uma referência de conformidade inicial, não um sandbox para código hostil. A memória é fornecida pelo chamador e a execução é síncrona. Ainda faltam formato binário congelado, ABI, inicialização explícita de registradores, semântica de tiles, vetores, interrupções, DMA, concorrência, compilador e driver físico.

## AX Bytecode container experimental v0

Foi adicionado um encoder/decoder binário versionado pelo cabeçalho ISA, com magic `AXBC`, inteiros little-endian, limite de 64 MiB, limite de 1.000.000 instruções, rejeição de opcode desconhecido, payload truncado e bytes excedentes. A decodificação valida o programa antes de retorná-lo.

Este formato é experimental e ainda não é ABI congelada. Não é ainda um formato de objeto executável com relocations, símbolos, debug info, assinatura criptográfica ou compatibilidade garantida entre versões. Cinco testes cobrem round-trip e rejeição de entradas inválidas.
