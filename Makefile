# LovelaceSharp — Makefile
# Targets: build, run, clean

PROJECT       := Lovelace.Console/Lovelace.Console.csproj
CONFIGURATION := Release
FRAMEWORK     := net10.0
PUBLISH_DIR   := Lovelace.Console/bin/$(CONFIGURATION)/$(FRAMEWORK)/publish

# Detect OS for binary extension
ifeq ($(OS),Windows_NT)
    BINARY := $(PUBLISH_DIR)/Lovelace.Console.exe
else
    BINARY := $(PUBLISH_DIR)/Lovelace.Console
endif

.PHONY: all build run clean

all: build

## build: Publish the console app in Release mode with full AOT-ready optimizations.
##        Output lands in $(PUBLISH_DIR).
build:
	dotnet publish $(PROJECT) \
		--configuration $(CONFIGURATION) \
		--framework $(FRAMEWORK) \
		--no-self-contained \
		-p:Optimize=true \
		-p:TieredCompilation=true \
		-p:TieredPGO=true \
		--output $(PUBLISH_DIR)

## run: Run the previously built binary (requires `make build` first).
run: $(BINARY)
	$(BINARY)

$(BINARY):
	@echo "Binary not found — run 'make build' first."
	@exit 1

## clean: Remove all build and publish artifacts.
clean:
	dotnet clean $(PROJECT) --configuration $(CONFIGURATION)
	@if exist "$(PUBLISH_DIR)" rmdir /s /q "$(PUBLISH_DIR)" 2>nul || rm -rf "$(PUBLISH_DIR)"

## help: List available targets.
help:
	@grep -E '^## ' Makefile | sed 's/^## //'
