import matplotlib.pyplot as plt
def parse_iperf_data(filename):
    parsed_data = []
    with open(filename, 'r') as file:
        for line in file:
            if 'sec' in line:
                parts = line.split()
                interval = parts[2]
                interval = int(float(interval.split("-")[1]))
                transfer = float((parts[4] + ' ' + parts[5]).split(" ")[0])
                bitrate = ((parts[6] + ' ' + parts[7]).split(" "))
                if(bitrate[1]=="Kbits/sec"):
                    bitrate = float(bitrate[0])/1000
                elif(bitrate[1]=="Gbits/sec"):
                    bitrate = float(bitrate[0])*1000
                elif(bitrate[1]=="Mbits/sec"):
                    bitrate = float(bitrate[0])
                parsed_data.append({'Time': interval, 'Transfer': transfer, 'Bitrate': bitrate})
    return parsed_data


# print(parsed_data[:-1])

def parse_quic_data(filename):
    parsed_data = []
    with open(filename) as file:
        for line in file:
            params = line.split(",")
            data_transferred = int(params[1].split(":")[1])/1000000
            time = int(params[0].split(":")[1])
            bandwidth = round(float(params[2].split(":")[1].strip()),2)
            parsed_data.append({'Time': time, 'Transfer': data_transferred, 'Bitrate': bandwidth})
    return parsed_data

# quic_data = parse_quic_data("rust-client/target/debug/noperf_quic_first.txt")

def plot_bandwidth(iperf_data,quic_data,time_clip=10000,caption="plot"):
    plt.rcParams.update({'font.size': 15})
    plt.figure(figsize=(12,6))
    time_iperf = []
    bandwidth_iperf = []
    time_quic = []
    bandwidth_quic = []
    for point in iperf_data:
        if(point['Time']>time_clip):
            break
        time_iperf.append(point['Time'])
        bandwidth_iperf.append(point['Bitrate'])
    for point in quic_data:
        if(point['Time']>time_clip):
            break
        time_quic.append(point['Time'])
        bandwidth_quic.append(point['Bitrate'])
    if(len(time_iperf)>len(time_quic)):
        for i in range(time_quic[-1],time_iperf[-1]):
            time_quic.append(time_iperf[i])
            bandwidth_quic.append(0)
    elif(len(time_quic)>len(time_iperf)):
        for i in range(time_iperf[-1],time_quic[-1]):
            time_iperf.append(time_quic[i])
            bandwidth_quic.append(0)

    zl_bandwidth = []
    for i in range(len(time_iperf)):
        zl_bandwidth.append(2.5)
    print(time_iperf)
    print(bandwidth_iperf)
    plt.plot(time_iperf,bandwidth_iperf,label="tcp")
    plt.plot(time_quic,bandwidth_quic,label="quic")
    plt.plot(time_iperf,zl_bandwidth,label="fairness_line",color='green',linestyle='dashed',linewidth=4)
    plt.xlabel("Time")
    plt.ylabel("Bandwidth")
    plt.legend(loc="upper left")
    plt.title(caption)
    plt.savefig(f"{caption}.pdf")
    plt.show()
    
#plot_bandwidth(iperf_data,quic_data,150,"QUIC First")
#plot_bandwidth(parse_iperf_data("iperf_tcp_first.txt"),parse_quic_data("rust-client/target/debug/noperf_tcp_first.txt"),caption="TCP First")
plot_bandwidth(parse_iperf_data("iperf_q_bbr_qf.txt"),parse_quic_data("rust-client/target/debug/bbr_quic_bw_qf.txt"),caption="quic_first_bbr")
plot_bandwidth(parse_iperf_data("iperf_q_bbr_tcpf.txt"),parse_quic_data("rust-client/target/debug/bbr_quic_bw_tcpf.txt"),caption="tcp_first_bbr")
plot_bandwidth(parse_iperf_data("iperf_q_cubic_qf.txt"),parse_quic_data("rust-client/target/debug/cubic_quic_bw_qf.txt"),caption="quic_first_cubic")
plot_bandwidth(parse_iperf_data("iperf_q_cubic_tcpf.txt"),parse_quic_data("rust-client/target/debug/cubic_quic_bw_tcpf.txt"),caption="tcp_first_cubic")
plot_bandwidth(parse_iperf_data("iperf_q_bbr2_qf.txt"),parse_quic_data("rust-client/target/debug/bbr2_quic_bw_qf.txt"),caption="quic_first_bbr2")
plot_bandwidth(parse_iperf_data("iperf_q_bbr2_tcpf.txt"),parse_quic_data("rust-client/target/debug/bbr2_quic_bw_tcpf.txt"),caption="tcp_first_bbr2")
plot_bandwidth(parse_iperf_data("iperf_q_reno_qf.txt"),parse_quic_data("rust-client/target/debug/reno_quic_bw_qf.txt"),caption="quic_first_reno")
plot_bandwidth(parse_iperf_data("iperf_q_reno_tcpf.txt"),parse_quic_data("rust-client/target/debug/reno_quic_bw_tcpf.txt"),caption="tcp_first_reno")