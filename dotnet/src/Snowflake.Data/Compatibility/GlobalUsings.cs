// TODO intended for future use
#if NET10_0_OR_GREATER
global using LockObject = System.Threading.Lock;
#endif

#if NET8_0 || NET9_0
global using LockObject = object;
#endif
