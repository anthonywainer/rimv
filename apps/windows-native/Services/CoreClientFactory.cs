using RimV.Windows.Application;

namespace RimV.Windows;

internal sealed class CoreClientFactory : ISharedCoreClientFactory
{
    public ISharedCoreClient Create(string recordingsDirectory, string modelsDirectory) =>
        CoreClient.Create(recordingsDirectory, modelsDirectory);
}
