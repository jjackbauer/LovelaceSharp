# LovelaceSharp — Makefile
# Targets: build, run, studio, test, clean, help

PROJECT        := Lovelace.Console/Lovelace.Console.csproj
STUDIO_PROJECT := Lovelace.Studio/Lovelace.Studio.csproj
CONFIGURATION  := Release
FRAMEWORK      := net10.0
PUBLISH_DIR    := Lovelace.Console/bin/$(CONFIGURATION)/$(FRAMEWORK)/publish

# Detect OS for binary extension
ifeq ($(OS),Windows_NT)
    BINARY := $(PUBLISH_DIR)/Lovelace.Console.exe
else
    BINARY := $(PUBLISH_DIR)/Lovelace.Console
endif

.PHONY: all build run studio test clean help

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

## run: Run the previously built console binary (requires `make build` first).
run: $(BINARY)
	$(BINARY)

## studio: Build and run the Lovelace.Studio web IDE (binds to localhost).
studio:
	dotnet run --project $(STUDIO_PROJECT)

## test: Run the fast test suites (skips the slow Lovelace.Real.Tests).
test:
	dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj
	dotnet test Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj
	dotnet test Lovelace.Natural.Tests/Lovelace.Natural.Tests.csproj
	dotnet test Lovelace.Integer.Tests/Lovelace.Integer.Tests.csproj
	dotnet test Lovelace.Representation.Tests/Lovelace.Representation.Tests.csproj

$(BINARY):
	@echo "Binary not found — run 'make build' first."
	@exit 1

## clean: Remove all build and publish artifacts.
clean:
	dotnet clean $(PROJECT) --configuration $(CONFIGURATION)
	dotnet clean $(STUDIO_PROJECT)
	@if exist "$(PUBLISH_DIR)" rmdir /s /q "$(PUBLISH_DIR)" 2>nul || rm -rf "$(PUBLISH_DIR)"

## help: List available targets.
help:
	@echo LovelaceSharp - targets:
	@echo   make build    Publish the console app (Release)
	@echo   make run      Run the previously built console binary
	@echo   make studio   Build + run the Lovelace.Studio web IDE
	@echo   make test     Run the fast test suites
	@echo   make clean    Remove build artifacts
