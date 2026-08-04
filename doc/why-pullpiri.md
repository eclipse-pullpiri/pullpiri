# **Pullpiri: Safety MEETS Developer Experience**

Pullpiri is an open-source container orchestration platform specialized for vehicle environments.

As modern vehicles integrate an increasing number of applications, overall software complexity continues to rise. Pullpiri adapts Kubernetes-based orchestration—proven in cloud environments—to automotive constraints, enabling rapid deployment and management of containerized applications across diverse hardware platforms.

Developers can deploy applications consistently to any vehicle using standardized containers, without complex cross-compilation or platform-specific optimization. Pullpiri also provides monitoring and automated recovery mechanisms that ensure stable container execution, supporting both vehicle safety and developer productivity.

---

## **Why Vehicle-Native Orchestration Matters**

Previously, deploying applications to vehicles required multiple steps including complex build processes, dependency management, and platform-specific optimization. Pullpiri simplifies this process, enabling consistent deployment to any vehicle platform in standardized container form.

Container orchestration in vehicles dramatically simplifies application deployment and management. However, vehicle container orchestration must consider vehicle-specific characteristics unlike general cloud environments. Pullpiri addresses these vehicle environment specificities as follows:

---

### **1. Safety-Level Constraints per SoC**

<div class="grid cards" markdown>

-   :material-alert-octagon: __Vehicle Environment Constraints__

    ---

    Unlike cloud orchestration—where nodes are treated equally—vehicle SoCs differ in safety levels. For example, entertainment apps must never be deployed on high-safety SoCs. Enforcing hardware safety policies is essential.

-   :material-check-decagram: __Solution__

    ---

    **Safety First: Automatic Safety-Level Policy Enforcement**

    Pullpiri automatically blocks deployments that violate safety-level constraints. This eliminates manual checking and prevents mis-deployments caused by human error.

</div>

---

### **2. Vehicle Signal-Based Decision Support**

<div class="grid cards" markdown>

-   :material-alert-octagon: __Vehicle Environment Constraints__

    ---

    Vehicles require application logic that reacts to continuously changing driving data. Therefore, support for in-vehicle communication protocols is critical.

-   :material-check-decagram: __Solution__

    ---

    **Driving Data-Driven Decision Making**

    Pullpiri supports CAN, FlexRay, and Automotive Ethernet, enabling runtime decisions based on real-time vehicle signals.

</div>

---

### **3. Integrated Container Management**

<div class="grid cards" markdown>

-   :material-alert-octagon: __Vehicle Environment Constraints__

    ---

    Many vehicle functions rely on multiple interdependent containers.

    For example, autonomous driving requires coordinated operation of sensors, planning, and control modules. A single container failure can cascade across the entire function.

-   :material-check-decagram: __Solution__

    ---

    **Cascade-Free Recovery: Dependency-Aware Sequential Deployment**

    Pullpiri installs and launches containers in correct dependency order and prevents cascade failures. It detects the functional impact of container errors, propagates state changes, and recovers the function as a whole.

</div>
