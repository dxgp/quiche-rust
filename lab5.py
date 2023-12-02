from mininet.net import Mininet
from mininet.link import TCLink
from mininet.node import Host,OVSKernelSwitch
from mininet.cli import CLI
from mininet.node import Node
from mininet.topo import Topo



def run_lab():
    net = Mininet(topo=None,link=TCLink,build=False)
    c0 = net.addController('c0',ip='127.0.0.1',port=6653)
    h1 = net.addHost('h1',cls=Host,ip='10.0.0.1/24',defaultRoute="dev h1-eth0")
    h2 = net.addHost('h2',cls=Host,ip='10.0.0.2/24',defaultRoute="dev h2-eth0")
    h3 = net.addHost('h3',cls=Host,ip='10.0.0.3/24',defaultRoute="dev h3-eth0")
    h4 = net.addHost('h4',cls=Host,ip='10.0.0.4/24',defaultRoute="dev h4-eth0")
    s1 = net.addSwitch('s1',cls=OVSKernelSwitch)
    s2 = net.addSwitch('s2',cls=OVSKernelSwitch)
    net.addLink(s1,s2,intfName="s1-eth1",intfName2="s2-eth1",bw=100,loss=2)
    net.addLink(h1,s1,intfName2="s1-eth3")
    net.addLink(h3,s1,intfName2="s1-eth2")
    net.addLink(h2,s2,intfName2="s2-eth2")
    net.addLink(h4,s2,intfName2="s2-eth3")
    c0.start()
    net.build()
    net.get("s1").start([c0])
    net.get("s2").start([c0])
    net.start()

    print(h2.cmd("cd rust-server"))
    pid = h2.cmd("./rust-server -- BBR &")
    print("PID ACT",pid)
    print("PID IS:",pid[4:])
    print(h1.cmd("cd rust-client/target/debug"))
    print(h1.cmd("./rust-client -- BBR"))
    print(h2.cmd("kill -9 "+pid[4:10]))


    pid = (h2.cmd("./rust-server -- BBR2 &"))
    print("PID IS:",pid[4:])
    print(h1.cmd("./rust-client -- BBR2"))
    print(h2.cmd("kill -9 "+pid[4:10]))

    pid = (h2.cmd("./rust-server -- CUBIC &"))
    print("PID IS:",pid[4:])
    print(h1.cmd("./rust-client -- CUBIC"))
    print(h2.cmd("kill -9 "+pid[4:10]))


    pid = (h2.cmd("./rust-server -- RENO &"))
    print("PID ACT:",pid)
    print("PID IS:",pid[4:])
    print(h1.cmd("./rust-client -- RENO"))
    print(h2.cmd("kill -9 "+pid[4:10]))

    CLI(net)
    net.stop()

if __name__=='__main__':
    run_lab()