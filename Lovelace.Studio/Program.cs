using System.Text.Json;
using Lovelace.Suite;
using Lovelace.Studio;

var builder = WebApplication.CreateBuilder(args);

// Single shared engine session (local single-user tool).
builder.Services.AddSingleton(new SuiteEngine());
builder.Services.AddSingleton<EngineHost>();

builder.Services.ConfigureHttpJsonOptions(options =>
{
    options.SerializerOptions.PropertyNamingPolicy = JsonNamingPolicy.CamelCase;
});

var app = builder.Build();

// Serve the IDE (index.html + wwwroot assets).
app.UseDefaultFiles();
app.UseStaticFiles();

app.MapPost("/api/evaluate", async (EvaluateRequest request, EngineHost host) =>
    Results.Ok(await host.EvaluateAsync(request.Source)));

app.MapGet("/api/state", (EngineHost host) => Results.Ok(host.GetState()));

app.MapDelete("/api/state", (EngineHost host) => Results.Ok(host.ClearVariables()));

app.MapDelete("/api/variables/{name}", (string name, EngineHost host) =>
    Results.Ok(host.DeleteVariable(name)));

app.Run();
