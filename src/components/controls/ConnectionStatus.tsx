interface ConnectionStatusProps {
    isConnected: boolean;
  }
  
  export default function ConnectionStatus(props: ConnectionStatusProps) {
    return (
      <div class={`fixed bottom-4 right-4 px-4 py-2 rounded-full ${
        props.isConnected ? 'bg-green-500' : 'bg-red-500'
      } text-white font-medium`}>
        {props.isConnected ? 'Connected' : 'Disconnected'}
      </div>
    );
  }