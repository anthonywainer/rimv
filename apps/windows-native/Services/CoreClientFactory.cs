using RimV.Windows.Application;

namespace RimV.Windows;

internal sealed class CoreClientFactory : ISharedCoreClientFactory
{
    private readonly IAppLog _log;

    public CoreClientFactory(IAppLog log) => _log = log;

    public ISharedCoreClient Create(string recordingsDirectory, string modelsDirectory) =>
        CoreClient.Create(recordingsDirectory, modelsDirectory, _log);
}
