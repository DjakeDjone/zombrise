using Newtonsoft.Json;
using System;
using System.Runtime.InteropServices;
using System.Threading;
using UnityEngine;

public class Suduxu : MonoBehaviour
{
    private Thread serverThread;

    [DllImport("suduxu",  CallingConvention = CallingConvention.Cdecl)]
    private static extern void start_suduxu();
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern void stop_suduxu();
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern bool is_running();
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern void disconnect_client(ushort id);
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern void disconnect_all();
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern bool get_button_in_state(ushort id, ButtonInputType type, ButtonInputState state);
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern SensorDataRaw get_sensor_data(ushort id);
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr find_all_clients();
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr find_client_by_id(ushort id);
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern void broadcast_tcp(IntPtr ptr);
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern void send_to_client(ushort id, IntPtr ptr);
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern void tick(float delta);
    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern void free(IntPtr ptr);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate void EventCallback(IntPtr ptr);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate void SensorEventCallback(ref SensorDataRaw data);

    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern void register_event_callback(EventCallback eventCallback);

    [DllImport("suduxu", CallingConvention = CallingConvention.Cdecl)]
    private static extern void register_sensor_event_callback(SensorEventCallback eventCallback);
    private static void OnEvent(IntPtr ptr)
    {
        string json = Marshal.PtrToStringAnsi(ptr);
        Debug.Log(json);

        free(ptr);
    }

    private static void OnSensorEvent(ref SensorDataRaw data)
    {
        Debug.Log(data);
    }



    private void Start()
    {
        register_event_callback(OnEvent);
        register_sensor_event_callback(OnSensorEvent);

        serverThread = new Thread(() => start_suduxu());
        serverThread.IsBackground = true;
        serverThread.Start();
    }

    private void Update()
    {
        if (get_button_in_state(1, ButtonInputType.A, ButtonInputState.Down))
        {
            Debug.Log("Pressed A down");
        }
        else if (Input.GetKeyDown(KeyCode.I))
        {
            Debug.Log(is_running());
        }
        else if (Input.GetKeyDown(KeyCode.D))
        {
            disconnect_client(1);
        }
        else if (Input.GetKeyDown(KeyCode.A))
        {
            disconnect_all();
        } else if (Input.GetKeyDown(KeyCode.F))
        {
            var ptr = find_all_clients();
            var clients = Marshal.PtrToStringAnsi(ptr);
            free(ptr);
            Debug.Log(clients);
        } else if (Input.GetKeyDown(KeyCode.L))
        {
            var ptr = find_client_by_id(1);
            var client = Marshal.PtrToStringAnsi(ptr);
            free(ptr);
            Debug.Log(client);
        }
        else if (Input.GetKeyDown(KeyCode.T))
        {
            Payload<Echo> payload = new Payload<Echo>(0, "Echo", new Echo("Broadcast"));
            string json = JsonConvert.SerializeObject(payload);
            IntPtr unmanaged = Marshal.StringToHGlobalAnsi(json);
            broadcast_tcp(unmanaged);
            Marshal.FreeHGlobal(unmanaged);
        } else if (Input.GetKeyDown(KeyCode.S))
        {
            Payload<Echo> payload = new Payload<Echo>(0, "Echo", new Echo("Unicast"));
            string json = JsonConvert.SerializeObject(payload);
            IntPtr unmanaged = Marshal.StringToHGlobalAnsi(json);
            send_to_client(1, unmanaged);
            Marshal.FreeHGlobal(unmanaged);
        }

        tick(Time.deltaTime);
    }

    private void OnDestroy()
    {
        StopServer();
    }

    private void StopServer()
    {
        stop_suduxu();

        if (serverThread != null && serverThread.IsAlive)
        {
            serverThread.Join();
            serverThread = null;
        }
    }
}