APP := block-screen-saver
CARGO ?= cargo

.PHONY: help check test build build-debug run clean

help:
	@echo "Targets disponíveis:"
	@echo "  make check                 Verifica o código"
	@echo "  make test                  Executa os testes"
	@echo "  make build                 Gera o binário otimizado (release)"
	@echo "  make build-debug           Gera o binário de desenvolvimento"
	@echo "  make run RUN_ARGS='...'    Executa com argumentos opcionais"
	@echo "  make clean                 Remove artefatos de compilação"

check:
	$(CARGO) check

test:
	$(CARGO) test

build:
	$(CARGO) build --release

build-debug:
	$(CARGO) build

run:
	$(CARGO) run -- $(RUN_ARGS)

clean:
	$(CARGO) clean
