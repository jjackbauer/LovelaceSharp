using System.Text.Json;
using Lovelace.Studio;
using Microsoft.AspNetCore.StaticFiles;

var builder = WebApplication.CreateBuilder(args);

// In-memory session registry: one independent session per browser tab.
builder.Services.AddSingleton<SessionRegistry>();
builder.Services.AddSingleton<EngineHost>();

builder.Services.ConfigureHttpJsonOptions(options =>
{
    options.SerializerOptions.PropertyNamingPolicy = JsonNamingPolicy.CamelCase;
    // Native AOT: resolve types from the source-generated context instead of
    // the (trimmed) reflection-based serializer.
    options.SerializerOptions.TypeInfoResolver = StudioJsonContext.Default;
});

var app = builder.Build();

// Serve the IDE (index.html + wwwroot assets). Local single-user tool: never
// let the browser cache the UI, so a rebuild is always reflected on refresh.
app.UseDefaultFiles();
app.UseStaticFiles(new StaticFileOptions
{
    OnPrepareResponse = ctx => ctx.Context.Response.Headers.CacheControl = "no-cache, no-store, must-revalidate"
});

static string? SessionId(HttpContext ctx) =>
    ctx.Request.Headers["X-Session-Id"].FirstOrDefault();

// Create a new session (the front-end calls this when it has no token yet).
app.MapPost("/api/session", (EngineHost host) =>
{
    var session = host.ResolveSession(null);
    return Results.Ok(new SessionResponse(session.Id, session.Precision, session.Engine.Revision));
});

// Resume/check an existing session.
app.MapGet("/api/session", (HttpContext ctx, EngineHost host) =>
{
    var session = host.TryGetSession(SessionId(ctx) ?? string.Empty);
    return session is null
        ? Results.NotFound()
        : Results.Ok(new SessionResponse(session.Id, session.Precision, session.Engine.Revision));
});

// Destroy a session.
app.MapDelete("/api/session", (HttpContext ctx, EngineHost host) =>
{
    var removed = host.RemoveSession(SessionId(ctx) ?? string.Empty);
    return removed ? Results.Ok() : Results.NotFound();
});

app.MapPost("/api/evaluate", (EvaluateRequest request, HttpContext ctx, EngineHost host) =>
{
    var session = host.TryGetSession(SessionId(ctx) ?? string.Empty);
    if (session is null)
        return Results.NotFound();
    try
    {
        return Results.Ok(host.StartRun(session, request.Source));
    }
    catch (InvalidOperationException ex)
    {
        return Results.Conflict(ex.Message);
    }
});

app.MapGet("/api/run/{runId}", (string runId, HttpContext ctx, EngineHost host) =>
{
    var session = host.TryGetSession(SessionId(ctx) ?? string.Empty);
    if (session is null)
        return Results.NotFound();
    var status = host.GetRun(session, runId);
    return status is null ? Results.NotFound() : Results.Ok(status);
});

app.MapPost("/api/run/{runId}/cancel", (string runId, HttpContext ctx, EngineHost host) =>
{
    var session = host.TryGetSession(SessionId(ctx) ?? string.Empty);
    if (session is null)
        return Results.NotFound();
    return host.CancelRun(session, runId) ? Results.Ok() : Results.NotFound();
});

app.MapPut("/api/precision", (SetPrecisionRequest request, HttpContext ctx, EngineHost host) =>
{
    var session = host.TryGetSession(SessionId(ctx) ?? string.Empty);
    if (session is null)
        return Results.NotFound();
    try
    {
        return Results.Ok(host.SetPrecision(session, request.Digits));
    }
    catch (InvalidOperationException ex)
    {
        return Results.BadRequest(ex.Message);
    }
});

app.MapGet("/api/completions", (HttpContext ctx, EngineHost host) =>
{
    var session = host.TryGetSession(SessionId(ctx) ?? string.Empty);
    if (session is null)
        return Results.NotFound();
    return Results.Ok(host.GetCompletions(session));
});

app.MapGet("/api/state", (HttpContext ctx, EngineHost host) =>
{
    var session = host.TryGetSession(SessionId(ctx) ?? string.Empty);
    if (session is null)
        return Results.NotFound();
    return Results.Ok(host.GetState(session));
});

app.MapDelete("/api/state", (HttpContext ctx, EngineHost host) =>
{
    var session = host.TryGetSession(SessionId(ctx) ?? string.Empty);
    if (session is null)
        return Results.NotFound();
    return Results.Ok(host.ClearVariables(session));
});

app.MapDelete("/api/variables/{name}", (string name, HttpContext ctx, EngineHost host) =>
{
    var session = host.TryGetSession(SessionId(ctx) ?? string.Empty);
    if (session is null)
        return Results.NotFound();
    return Results.Ok(host.DeleteVariable(session, name));
});

app.Run();
